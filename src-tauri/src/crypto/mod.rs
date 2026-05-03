mod provider;

use crate::app_state::ModuleDescriptor;

pub use provider::{
    key_fingerprint, DefaultCryptoProvider, EncryptedPayload, EncryptionKeyMaterial,
    ProtocolCryptoProvider, SigningKeyMaterial,
};

pub const CRYPTO_STACK: [&str; 3] = ["ed25519-dalek", "x25519-dalek", "chacha20poly1305"];

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "crypto",
        responsibility:
            "Provides signing, verification, encrypted private delivery, and key identity abstractions for the protocol layer.",
    }
}

#[must_use]
pub fn stack() -> Vec<&'static str> {
    CRYPTO_STACK.to_vec()
}
