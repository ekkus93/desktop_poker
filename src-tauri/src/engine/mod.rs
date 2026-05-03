use crate::app_state::ModuleDescriptor;

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "engine",
        responsibility:
            "Evaluates poker hands, betting legality, and hand settlement without letting the frontend become the source of truth.",
    }
}
