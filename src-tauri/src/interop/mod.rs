use crate::app_state::ModuleDescriptor;

pub const JOIN_PAYLOAD_ENCODING: &str =
    "pkr1_ compact join payload (with optional legacy raw JSON support later)";

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "interop",
        responsibility:
            "Tracks Android compatibility fixtures, field names, runtime defaults, and any intentional temporary interoperability gaps.",
    }
}
