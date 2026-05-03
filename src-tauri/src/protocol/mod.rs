use crate::app_state::ModuleDescriptor;

pub const PROTOCOL_VERSION: u32 = 1;
pub const SERIALIZATION_STRATEGY: &str = "serde models plus canonical JSON bytes for signing";

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "protocol",
        responsibility:
            "Owns Android-compatible envelope models, canonical serialization, snapshots, signed events, and join payload schemas.",
    }
}
