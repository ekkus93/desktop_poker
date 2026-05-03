mod framing;
mod runtime;

use crate::app_state::ModuleDescriptor;

pub use framing::{read_json_frame, write_json_frame};
pub use runtime::{
    resolve_connectable_host_ip, ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent,
    HostRuntimeConfig, HostRuntimeMode, HostServer, NetworkingError,
};

pub const DEFAULT_HOST_PORT: u16 = 43_818;
pub const FRAMING_STRATEGY: &str = "length-prefixed JSON envelopes";
pub const RUNTIME_TRANSPORT: &str = "raw TCP over LAN";
pub const ROOM_CODE_DISCOVERY_DEFERRED_NOTE: &str =
    "TODO(networking): room-code discovery remains deferred and must stay out of the production UI until it is a real runtime path.";

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "networking",
        responsibility:
            "Owns TCP listener/client runtime, framing, LAN address validation, reconnect transport, and multi-instance-safe connection behavior.",
    }
}
