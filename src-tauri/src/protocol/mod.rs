mod canonical;
mod join_payload;
mod models;
mod replay;

use crate::app_state::ModuleDescriptor;

pub use canonical::{
    canonical_json_bytes, canonical_json_bytes_without_signature, CanonicalJsonFixture,
};
pub use join_payload::{decode_join_payload, encode_join_payload, validate_join_payload};
pub use models::*;
pub use replay::{ReplayProtector, ServerSequenceTracker};

pub const PROTOCOL_VERSION: u32 = 1;
pub const SERIALIZATION_STRATEGY: &str = "serde models plus canonical JSON bytes for signing";
pub const JOIN_PAYLOAD_PREFIX: &str = "pkr1_";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolError {
    message: String,
}

impl ProtocolError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ProtocolError {}

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "protocol",
        responsibility:
            "Owns Android-compatible envelope models, canonical serialization, snapshots, signed events, and join payload schemas.",
    }
}
