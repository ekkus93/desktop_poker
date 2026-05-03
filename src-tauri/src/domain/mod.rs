use crate::app_state::ModuleDescriptor;

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "domain",
        responsibility:
            "Defines poker, tournament, seat, participant, and projection value types shared across the runtime.",
    }
}
