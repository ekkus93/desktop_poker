use crate::app_state::ModuleDescriptor;

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "tournament",
        responsibility:
            "Coordinates roster freeze, blind scheduling, hand loops, eliminations, and tournament completion.",
    }
}
