use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

use super::{derive_chacha_key, key_fingerprint, DefaultCryptoProvider, ProtocolCryptoProvider};

// ----- 1.1 Signing round-trip (Ed25519) -----

#[test]
fn sign_then_verify_round_trips() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();
    let message = b"canonical signing bytes";

    let signature = provider.sign(&keys, message);

    assert!(provider
        .verify(&keys.public_key_base64(), message, &signature)
        .is_ok());
}

#[test]
fn signing_is_deterministic_for_same_key_and_message() {
    // Ed25519 signatures are deterministic; identical inputs must produce an
    // identical encoded signature so canonical signing bytes stay stable.
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();
    let message = b"deterministic payload";

    let first = provider.sign(&keys, message);
    let second = provider.sign(&keys, message);

    assert_eq!(first, second);
}

#[test]
fn distinct_keypairs_produce_distinct_public_keys() {
    let provider = DefaultCryptoProvider;
    let a = provider.generate_signing_keypair();
    let b = provider.generate_signing_keypair();

    assert_ne!(a.public_key_base64(), b.public_key_base64());
    assert_ne!(a.key_id(), b.key_id());
}

// ----- 1.2 Signature verification — negative cases -----

#[test]
fn verify_rejects_tampered_message() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();
    let signature = provider.sign(&keys, b"original message");

    assert!(provider
        .verify(&keys.public_key_base64(), b"tampered message", &signature)
        .is_err());
}

#[test]
fn verify_rejects_wrong_public_key() {
    let provider = DefaultCryptoProvider;
    let signer = provider.generate_signing_keypair();
    let other = provider.generate_signing_keypair();
    let message = b"signed by signer";
    let signature = provider.sign(&signer, message);

    assert!(provider
        .verify(&other.public_key_base64(), message, &signature)
        .is_err());
}

#[test]
fn verify_rejects_tampered_signature() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();
    let message = b"signed message";
    let signature = provider.sign(&keys, message);

    // Flip one base64url character; the signature stays 64 bytes but no longer
    // matches the message.
    let mut chars: Vec<char> = signature.chars().collect();
    chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    assert!(provider
        .verify(&keys.public_key_base64(), message, &tampered)
        .is_err());
}

#[test]
fn verify_rejects_malformed_signature_encoding() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();

    let result = provider.verify(&keys.public_key_base64(), b"message", "not*base64*url");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("signature"));
}

#[test]
fn verify_rejects_malformed_verifying_key() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();
    let signature = provider.sign(&keys, b"message");

    // A 16-byte key cannot decode to the required 32 bytes.
    let short_key = URL_SAFE_NO_PAD.encode([7_u8; 16]);
    let result = provider.verify(&short_key, b"message", &signature);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("verifying key"));
}

// ----- 1.3 Encryption round-trip (X25519 + ChaCha20-Poly1305) -----

#[test]
fn encrypt_then_decrypt_round_trips() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let plaintext = b"hole cards: As Kd";
    let aad = b"table-1:hand-7";

    let payload = provider
        .encrypt(&sender, &recipient.public_key_base64(), plaintext, aad)
        .expect("encryption should succeed");
    let decrypted = provider
        .decrypt(&recipient, &sender.public_key_base64(), &payload, aad)
        .expect("decryption should succeed");

    assert_eq!(decrypted, plaintext);
}

#[test]
fn encrypted_payload_carries_recipient_fingerprint() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();

    let payload = provider
        .encrypt(&sender, &recipient.public_key_base64(), b"secret", b"aad")
        .expect("encryption should succeed");

    assert_eq!(payload.recipient_key_id, recipient.key_id());
}

#[test]
fn nonce_and_ciphertext_are_base64url_no_padding() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();

    let payload = provider
        .encrypt(&sender, &recipient.public_key_base64(), b"secret", b"aad")
        .expect("encryption should succeed");

    assert!(!payload.nonce_base64.contains('='));
    assert!(!payload.ciphertext_base64.contains('='));
    assert!(URL_SAFE_NO_PAD
        .decode(payload.nonce_base64.as_bytes())
        .is_ok());
    assert!(URL_SAFE_NO_PAD
        .decode(payload.ciphertext_base64.as_bytes())
        .is_ok());
}

#[test]
fn each_encryption_uses_a_fresh_nonce() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let recipient_pub = recipient.public_key_base64();

    let first = provider
        .encrypt(&sender, &recipient_pub, b"same plaintext", b"aad")
        .expect("encryption should succeed");
    let second = provider
        .encrypt(&sender, &recipient_pub, b"same plaintext", b"aad")
        .expect("encryption should succeed");

    assert_ne!(first.nonce_base64, second.nonce_base64);
    assert_ne!(first.ciphertext_base64, second.ciphertext_base64);
}

// ----- 1.4 Decryption — negative cases -----

#[test]
fn decrypt_rejects_tampered_ciphertext() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let mut payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad",
        )
        .expect("encryption should succeed");

    let mut chars: Vec<char> = payload.ciphertext_base64.chars().collect();
    chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
    payload.ciphertext_base64 = chars.into_iter().collect();

    assert!(provider
        .decrypt(&recipient, &sender.public_key_base64(), &payload, b"aad")
        .is_err());
}

#[test]
fn decrypt_rejects_wrong_sender_key() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let impostor = provider.generate_encryption_keypair();
    let payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad",
        )
        .expect("encryption should succeed");

    // The DH agreement won't match if the sender key is wrong.
    assert!(provider
        .decrypt(&recipient, &impostor.public_key_base64(), &payload, b"aad")
        .is_err());
}

#[test]
fn decrypt_rejects_wrong_recipient_key() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let third_party = provider.generate_encryption_keypair();
    let payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad",
        )
        .expect("encryption should succeed");

    assert!(provider
        .decrypt(&third_party, &sender.public_key_base64(), &payload, b"aad")
        .is_err());
}

#[test]
fn decrypt_rejects_mismatched_aad() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad-A",
        )
        .expect("encryption should succeed");

    assert!(provider
        .decrypt(&recipient, &sender.public_key_base64(), &payload, b"aad-B")
        .is_err());
}

#[test]
fn decrypt_rejects_tampered_or_wrong_length_nonce() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let mut payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad",
        )
        .expect("encryption should succeed");

    // An 11-byte nonce cannot decode to the required 12 bytes.
    payload.nonce_base64 = URL_SAFE_NO_PAD.encode([0_u8; 11]);

    let result = provider.decrypt(&recipient, &sender.public_key_base64(), &payload, b"aad");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("nonce"));
}

#[test]
fn decrypt_rejects_malformed_ciphertext_encoding() {
    let provider = DefaultCryptoProvider;
    let sender = provider.generate_encryption_keypair();
    let recipient = provider.generate_encryption_keypair();
    let mut payload = provider
        .encrypt(
            &sender,
            &recipient.public_key_base64(),
            b"hole cards",
            b"aad",
        )
        .expect("encryption should succeed");

    payload.ciphertext_base64 = "not*base64*url".to_string();

    let result = provider.decrypt(&recipient, &sender.public_key_base64(), &payload, b"aad");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("ciphertext"));
}

// ----- 1.5 Key material + fingerprint format -----

#[test]
fn public_key_base64_is_url_safe_no_pad() {
    let provider = DefaultCryptoProvider;
    let signing = provider.generate_signing_keypair();
    let encryption = provider.generate_encryption_keypair();

    for encoded in [signing.public_key_base64(), encryption.public_key_base64()] {
        assert!(!encoded.contains('='));
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded.as_bytes())
            .expect("public key should decode");
        assert_eq!(decoded.len(), 32);
    }
}

#[test]
fn key_fingerprint_is_16_lowercase_hex() {
    let key_bytes = [9_u8; 32];
    let fingerprint = key_fingerprint(&key_bytes);

    assert_eq!(fingerprint.len(), 16);
    assert!(fingerprint
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));

    // Android contract: first 8 bytes of SHA-256 of the key, lowercase hex.
    let expected = hex::encode(&Sha256::digest(key_bytes)[..8]);
    assert_eq!(fingerprint, expected);
}

#[test]
fn fingerprint_is_stable_for_same_key() {
    let key_bytes = [3_u8; 32];

    assert_eq!(key_fingerprint(&key_bytes), key_fingerprint(&key_bytes));
}

// ----- 1.6 `decode_exact` helper edge cases (via public methods) -----

#[test]
fn decode_error_messages_include_the_field_label() {
    let provider = DefaultCryptoProvider;

    // Wrong-length-but-valid base64 exercises the "must decode to N bytes" path.
    let wrong_length_key = URL_SAFE_NO_PAD.encode([1_u8; 10]);
    let key_error = provider
        .verify(&wrong_length_key, b"m", &"A".repeat(86))
        .unwrap_err()
        .to_string();
    assert!(key_error.contains("verifying key"));
    assert!(key_error.contains("32 bytes"));
}

#[test]
fn wrong_length_valid_base64_signature_is_rejected_with_byte_count_message() {
    let provider = DefaultCryptoProvider;
    let keys = provider.generate_signing_keypair();

    // Valid base64url that decodes to 8 bytes, not the required 64.
    let short_signature = URL_SAFE_NO_PAD.encode([0_u8; 8]);
    let error = provider
        .verify(&keys.public_key_base64(), b"message", &short_signature)
        .unwrap_err()
        .to_string();

    assert!(error.contains("signature"));
    assert!(error.contains("64 bytes"));
}

// ----- existing tests (kept) -----

#[test]
fn fingerprints_are_16_hex_chars() {
    let provider = DefaultCryptoProvider;
    let signing_keys = provider.generate_signing_keypair();

    assert_eq!(signing_keys.key_id().len(), 16);
}

#[test]
fn chacha_key_derivation_matches_android_hkdf_contract() {
    let shared_secret: Vec<u8> = (0_u8..32).collect();

    let derived_key = derive_chacha_key(&shared_secret);

    assert_eq!(
        hex::encode(derived_key),
        "10525480707918b34d47aacb80882368c9a76ff98040097de3aadd17687de844"
    );
}
