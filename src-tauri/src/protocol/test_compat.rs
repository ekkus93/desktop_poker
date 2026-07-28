use super::ProtocolMessageType;

impl ProtocolMessageType {
    #[allow(non_upper_case_globals)]
    pub const HandEndedEvent: Self = Self::HandResultCommittedEvent;
}
