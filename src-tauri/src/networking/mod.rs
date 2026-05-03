use crate::app_state::ModuleDescriptor;

pub const DEFAULT_HOST_PORT: u16 = 43_818;
pub const FRAMING_STRATEGY: &str = "length-prefixed JSON envelopes";
pub const RUNTIME_TRANSPORT: &str = "raw TCP over LAN";

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "networking",
        responsibility:
            "Owns TCP listener/client runtime, framing, LAN address validation, reconnect transport, and multi-instance-safe connection behavior.",
    }
}
