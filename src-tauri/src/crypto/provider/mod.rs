use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use crate::protocol::ProtocolError;

#[derive(Debug)]
pub struct SigningKeyMaterial {
    signing_key: SigningKey,
}

impl SigningKeyMaterial {
    #[must_use]
    pub fn public_key_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.signing_key.verifying_key().as_bytes())
    }

    #[must_use]
    pub fn key_id(&self) -> String {
        key_fingerprint(self.signing_key.verifying_key().as_bytes())
    }
}

pub struct EncryptionKeyMaterial {
    private_key: StaticSecret,
    public_key: X25519PublicKey,
}

impl std::fmt::Debug for EncryptionKeyMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionKeyMaterial")
            .field("public_key_base64", &self.public_key_base64())
            .field("key_id", &self.key_id())
            .finish()
    }
}

impl EncryptionKeyMaterial {
    #[must_use]
    pub fn public_key_base64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.public_key.as_bytes())
    }

    #[must_use]
    pub fn key_id(&self) -> String {
        key_fingerprint(self.public_key.as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncryptedPayload {
    pub nonce_base64: String,
    pub ciphertext_base64: String,
    pub recipient_key_id: String,
}

const ANDROID_KDF_SALT: &[u8] = b"android_poker:v1";
const ANDROID_KDF_INFO: &[u8] = b"x25519-chacha20poly1305";

pub trait ProtocolCryptoProvider {
    fn generate_signing_keypair(&self) -> SigningKeyMaterial;
    fn generate_encryption_keypair(&self) -> EncryptionKeyMaterial;
    fn sign(&self, key_material: &SigningKeyMaterial, bytes: &[u8]) -> String;
    fn verify(
        &self,
        verifying_key_base64: &str,
        bytes: &[u8],
        signature_base64: &str,
    ) -> Result<(), ProtocolError>;
    fn encrypt(
        &self,
        sender_key_material: &EncryptionKeyMaterial,
        recipient_public_key_base64: &str,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<EncryptedPayload, ProtocolError>;
    fn decrypt(
        &self,
        recipient_key_material: &EncryptionKeyMaterial,
        sender_public_key_base64: &str,
        encrypted_payload: &EncryptedPayload,
        associated_data: &[u8],
    ) -> Result<Vec<u8>, ProtocolError>;
}

#[derive(Clone, Copy, Default)]
pub struct DefaultCryptoProvider;

impl ProtocolCryptoProvider for DefaultCryptoProvider {
    fn generate_signing_keypair(&self) -> SigningKeyMaterial {
        let mut rng = OsRng;
        SigningKeyMaterial {
            signing_key: SigningKey::generate(&mut rng),
        }
    }

    fn generate_encryption_keypair(&self) -> EncryptionKeyMaterial {
        let mut rng = OsRng;
        let mut bytes = [0_u8; 32];
        rng.fill_bytes(&mut bytes);
        let private_key = StaticSecret::from(bytes);
        let public_key = X25519PublicKey::from(&private_key);

        EncryptionKeyMaterial {
            private_key,
            public_key,
        }
    }

    fn sign(&self, key_material: &SigningKeyMaterial, bytes: &[u8]) -> String {
        let signature = key_material.signing_key.sign(bytes);
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    }

    fn verify(
        &self,
        verifying_key_base64: &str,
        bytes: &[u8],
        signature_base64: &str,
    ) -> Result<(), ProtocolError> {
        let verifying_key_bytes = decode_exact::<32>(verifying_key_base64, "verifying key")?;
        let signature_bytes = decode_exact::<64>(signature_base64, "signature")?;
        let verifying_key = VerifyingKey::from_bytes(&verifying_key_bytes)
            .map_err(|error| ProtocolError::new(format!("invalid verifying key: {error}")))?;
        let signature = Signature::from_bytes(&signature_bytes);

        verifying_key
            .verify(bytes, &signature)
            .map_err(|error| ProtocolError::new(format!("signature verification failed: {error}")))
    }

    fn encrypt(
        &self,
        sender_key_material: &EncryptionKeyMaterial,
        recipient_public_key_base64: &str,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<EncryptedPayload, ProtocolError> {
        let recipient_public_key_bytes =
            decode_exact::<32>(recipient_public_key_base64, "recipient public key")?;
        let recipient_public_key = X25519PublicKey::from(recipient_public_key_bytes);
        let shared_secret = sender_key_material
            .private_key
            .diffie_hellman(&recipient_public_key);
        let cipher = ChaCha20Poly1305::new(&derive_chacha_key(shared_secret.as_bytes()));

        let mut nonce_bytes = [0_u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);

        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad: associated_data,
                },
            )
            .map_err(|error| ProtocolError::new(format!("encryption failed: {error}")))?;

        Ok(EncryptedPayload {
            nonce_base64: URL_SAFE_NO_PAD.encode(nonce_bytes),
            ciphertext_base64: URL_SAFE_NO_PAD.encode(ciphertext),
            recipient_key_id: key_fingerprint(recipient_public_key.as_bytes()),
        })
    }

    fn decrypt(
        &self,
        recipient_key_material: &EncryptionKeyMaterial,
        sender_public_key_base64: &str,
        encrypted_payload: &EncryptedPayload,
        associated_data: &[u8],
    ) -> Result<Vec<u8>, ProtocolError> {
        let sender_public_key_bytes =
            decode_exact::<32>(sender_public_key_base64, "sender public key")?;
        let sender_public_key = X25519PublicKey::from(sender_public_key_bytes);
        let shared_secret = recipient_key_material
            .private_key
            .diffie_hellman(&sender_public_key);
        let cipher = ChaCha20Poly1305::new(&derive_chacha_key(shared_secret.as_bytes()));

        let nonce_bytes = decode_exact::<12>(&encrypted_payload.nonce_base64, "nonce")?;
        let ciphertext = URL_SAFE_NO_PAD
            .decode(encrypted_payload.ciphertext_base64.as_bytes())
            .map_err(|error| ProtocolError::new(format!("invalid ciphertext encoding: {error}")))?;

        cipher
            .decrypt(
                Nonce::from_slice(&nonce_bytes),
                chacha20poly1305::aead::Payload {
                    msg: &ciphertext,
                    aad: associated_data,
                },
            )
            .map_err(|error| ProtocolError::new(format!("decryption failed: {error}")))
    }
}

#[must_use]
pub fn key_fingerprint(public_key_bytes: &[u8]) -> String {
    let digest = Sha256::digest(public_key_bytes);
    hex::encode(&digest[..8])
}

fn derive_chacha_key(shared_secret_bytes: &[u8]) -> Key {
    let hkdf = Hkdf::<Sha256>::new(Some(ANDROID_KDF_SALT), shared_secret_bytes);
    let mut key_bytes = [0_u8; 32];
    hkdf.expand(ANDROID_KDF_INFO, &mut key_bytes)
        .expect("HKDF expand for a 32-byte ChaCha key should not fail");
    *Key::from_slice(&key_bytes)
}

fn decode_exact<const N: usize>(encoded: &str, label: &str) -> Result<[u8; N], ProtocolError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded.as_bytes())
        .map_err(|error| ProtocolError::new(format!("invalid {label} encoding: {error}")))?;

    decoded
        .try_into()
        .map_err(|_| ProtocolError::new(format!("{label} must decode to {N} bytes")))
}

#[cfg(test)]
mod tests;
