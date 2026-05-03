use crate::app_state::ModuleDescriptor;

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "storage",
        responsibility:
            "Persists per-instance settings, reconnect identity material, and local desktop state without cross-instance stomping.",
    }
}
