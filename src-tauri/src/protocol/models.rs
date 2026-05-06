use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    crypto::{EncryptedPayload, ProtocolCryptoProvider, SigningKeyMaterial},
    domain::{
        ActionType, Card, HandResult, JoinPayload, PlacementEntry, PublicState, SnapshotState,
    },
};

use super::{
    canonical_json_bytes, canonical_json_bytes_without_signature, validate_join_payload,
    ProtocolError, PROTOCOL_VERSION,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtocolMessageType {
    JoinTournamentRequest,
    ReconnectTournamentRequest,
    SeatClaimRequest,
    ReadyStateRequest,
    TournamentStartedEvent,
    HandStartingEvent,
    ActionWindowOpenedEvent,
    PlayerActionCommittedEvent,
    StreetRevealedEvent,
    ShowdownStartedEvent,
    ShowdownHandRevealedEvent,
    HandResultCommittedEvent,
    HandLifecycleEvent,
    ActionSubmissionRequest,
    ActionRejectedEvent,
    EliminationEvent,
    TournamentCompleteEvent,
    SnapshotEvent,
    ResyncRequest,
    ProtocolError,
    PrivateHoleCardsEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedEnvelope<TPayload> {
    pub protocol_version: u32,
    pub message_type: ProtocolMessageType,
    pub table_id: String,
    pub session_epoch: u64,
    pub sender_id: String,
    pub counter: u64,
    pub message_id: String,
    pub server_sequence: Option<u64>,
    pub payload: TPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl<TPayload> SignedEnvelope<TPayload>
where
    TPayload: Serialize + Clone,
{
    pub fn sign(
        &mut self,
        crypto_provider: &impl ProtocolCryptoProvider,
        signing_keys: &SigningKeyMaterial,
    ) -> Result<(), ProtocolError> {
        let bytes = canonical_json_bytes_without_signature(self)?;
        self.signature = Some(crypto_provider.sign(signing_keys, &bytes));
        Ok(())
    }

    pub fn verify(
        &self,
        crypto_provider: &impl ProtocolCryptoProvider,
        verifying_key_base64: &str,
    ) -> Result<(), ProtocolError> {
        let signature = self
            .signature
            .as_deref()
            .ok_or_else(|| ProtocolError::new("signed envelope missing signature"))?;
        let bytes = canonical_json_bytes_without_signature(self)?;
        crypto_provider.verify(verifying_key_base64, &bytes, signature)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedPrivateEnvelope {
    pub protocol_version: u32,
    pub message_type: ProtocolMessageType,
    pub table_id: String,
    pub session_epoch: u64,
    pub sender_id: String,
    pub counter: u64,
    pub message_id: String,
    pub server_sequence: u64,
    pub recipient_id: String,
    pub recipient_key_id: String,
    pub nonce: String,
    pub ciphertext: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl EncryptedPrivateEnvelope {
    pub fn associated_data_json(&self) -> Result<Vec<u8>, ProtocolError> {
        canonical_json_bytes(&serde_json::json!({
            "protocolVersion": self.protocol_version,
            "messageType": self.message_type,
            "tableId": self.table_id,
            "sessionEpoch": self.session_epoch,
            "senderId": self.sender_id,
            "counter": self.counter,
            "messageId": self.message_id,
            "serverSequence": self.server_sequence,
            "recipientId": self.recipient_id,
            "recipientKeyId": self.recipient_key_id
        }))
    }

    pub fn sign(
        &mut self,
        crypto_provider: &impl ProtocolCryptoProvider,
        signing_keys: &SigningKeyMaterial,
    ) -> Result<(), ProtocolError> {
        let bytes = canonical_json_bytes_without_signature(self)?;
        self.signature = Some(crypto_provider.sign(signing_keys, &bytes));
        Ok(())
    }

    pub fn verify(
        &self,
        crypto_provider: &impl ProtocolCryptoProvider,
        verifying_key_base64: &str,
    ) -> Result<(), ProtocolError> {
        let signature = self
            .signature
            .as_deref()
            .ok_or_else(|| ProtocolError::new("private envelope missing signature"))?;
        let bytes = canonical_json_bytes_without_signature(self)?;
        crypto_provider.verify(verifying_key_base64, &bytes, signature)
    }

    pub fn from_encrypted_payload(
        payload: EncryptedPayload,
        metadata: PrivateEnvelopeMetadata,
    ) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::PrivateHoleCardsEvent,
            table_id: metadata.table_id,
            session_epoch: metadata.session_epoch,
            sender_id: metadata.sender_id,
            counter: metadata.counter,
            message_id: metadata.message_id,
            server_sequence: metadata.server_sequence,
            recipient_id: metadata.recipient_id,
            recipient_key_id: payload.recipient_key_id,
            nonce: payload.nonce_base64,
            ciphertext: payload.ciphertext_base64,
            signature: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateEnvelopeMetadata {
    pub sender_id: String,
    pub table_id: String,
    pub session_epoch: u64,
    pub counter: u64,
    pub message_id: String,
    pub server_sequence: u64,
    pub recipient_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JoinTournamentRequest {
    pub display_name: String,
    pub join_token: String,
    pub signing_public_key: String,
    pub encryption_public_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectTournamentRequest {
    pub player_id: String,
    pub reconnect_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_known_server_seq: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeatClaimRequest {
    pub seat_index: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadyStateRequest {
    pub is_ready: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TournamentStartedEvent {
    pub tournament_name: String,
    pub starting_stack: u32,
    pub blind_schedule_preset: String,
    pub frozen_player_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandStartingEvent {
    pub hand_number: u32,
    pub hand_phase: String,
    pub dealer_seat_index: u8,
    pub small_blind_seat_index: u8,
    pub big_blind_seat_index: u8,
    pub board_cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionWindowOpened {
    pub hand_number: u32,
    pub hand_phase: String,
    pub action_window_id: String,
    pub player_id: String,
    pub seat_index: u8,
    pub legal_actions: Vec<ActionType>,
    pub call_amount: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_raise_to: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raise_to: Option<u32>,
    pub deadline_epoch_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerActionCommitted {
    pub hand_number: u32,
    pub seat_index: u8,
    pub player_id: String,
    pub action_type: ActionType,
    pub raise_to_amount: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StreetRevealed {
    pub hand_number: u32,
    pub street: String,
    pub board_cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShowdownStarted {
    pub hand_number: u32,
    pub board_cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShowdownHandRevealed {
    pub hand_number: u32,
    pub player_id: String,
    pub hole_cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandResultCommitted {
    pub hand_number: u32,
    pub result: HandResult,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerActionSubmission {
    pub action_window_id: String,
    pub seat_index: u8,
    pub action_type: ActionType,
    pub raise_to_amount: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActionRejectedEvent {
    pub seat_index: u8,
    pub action_type: ActionType,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EliminationEvent {
    pub player_id: String,
    pub place: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TournamentCompleteEvent {
    pub winner_player_id: String,
    pub placements: Vec<PlacementEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResyncRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_server_sequence: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolErrorMessage {
    pub code: String,
    pub message: String,
    pub rejected_message_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateHoleCardsEvent {
    pub recipient_player_id: String,
    pub hole_cards: Vec<Card>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEvent {
    pub state: SnapshotState,
    pub local_player_id: String,
    pub reconnect_token: Option<String>,
    pub host_signing_public_key: Option<String>,
    pub host_encryption_public_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicTableSnapshot {
    pub tournament_name: String,
    pub room_code: Option<String>,
    pub seats: Vec<crate::domain::SeatState>,
    pub board_cards: Vec<Card>,
    pub blind_level_label: Option<String>,
    pub current_hand_number: Option<u32>,
    pub placements: Vec<PlacementEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPrivateProjection {
    pub public_snapshot: PublicState,
    pub local_player_id: String,
    pub private_hole_cards: Vec<Card>,
    pub can_act: bool,
    pub is_observer: bool,
    pub action_window_player_id: Option<String>,
}

pub type JsonSignedEnvelope = SignedEnvelope<Value>;

pub fn join_request_envelope(
    table_id: String,
    session_epoch: u64,
    sender_id: String,
    counter: u64,
    message_id: String,
    request: JoinTournamentRequest,
) -> SignedEnvelope<JoinTournamentRequest> {
    SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::JoinTournamentRequest,
        table_id,
        session_epoch,
        sender_id,
        counter,
        message_id,
        server_sequence: None,
        payload: request,
        signature: None,
    }
}

pub fn join_payload_to_json_value(payload: &JoinPayload) -> Result<Value, ProtocolError> {
    validate_join_payload(payload)?;

    serde_json::to_value(payload).map_err(|error| ProtocolError::new(error.to_string()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        canonical_json_bytes_without_signature, ActionWindowOpened, EncryptedPrivateEnvelope,
        PlayerActionCommitted, PrivateEnvelopeMetadata, ProtocolMessageType,
        ReconnectTournamentRequest, SignedEnvelope,
    };
    use crate::{
        crypto::{DefaultCryptoProvider, EncryptedPayload, ProtocolCryptoProvider},
        domain::ActionType,
        protocol::PROTOCOL_VERSION,
    };

    fn sample_signed_envelope() -> SignedEnvelope<serde_json::Value> {
        SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::SnapshotEvent,
            table_id: "table-1".to_string(),
            session_epoch: 4,
            sender_id: "host".to_string(),
            counter: 7,
            message_id: "message-1".to_string(),
            server_sequence: None,
            payload: json!({ "value": 7 }),
            signature: None,
        }
    }

    fn sample_private_envelope() -> EncryptedPrivateEnvelope {
        EncryptedPrivateEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::PrivateHoleCardsEvent,
            table_id: "table-1".to_string(),
            session_epoch: 4,
            sender_id: "host".to_string(),
            counter: 8,
            message_id: "private-1".to_string(),
            server_sequence: 9,
            recipient_id: "player-1".to_string(),
            recipient_key_id: "recipient-key".to_string(),
            nonce: "nonce".to_string(),
            ciphertext: "ciphertext".to_string(),
            signature: None,
        }
    }

    #[test]
    fn signed_envelope_sign_populates_signature_and_verify_checks_bytes() {
        let provider = DefaultCryptoProvider;
        let signing_keys = provider.generate_signing_keypair();
        let verifying_key = signing_keys.public_key_base64();
        let mut envelope = sample_signed_envelope();

        envelope
            .sign(&provider, &signing_keys)
            .expect("sign envelope");

        assert!(envelope.signature.is_some());
        envelope
            .verify(&provider, &verifying_key)
            .expect("signature should verify");

        let mut mutated = envelope.clone();
        mutated.payload = json!({ "value": 8 });
        assert!(mutated.verify(&provider, &verifying_key).is_err());
    }

    #[test]
    fn signed_envelope_verify_fails_when_signature_is_missing() {
        let provider = DefaultCryptoProvider;
        let signing_keys = provider.generate_signing_keypair();

        let error = sample_signed_envelope()
            .verify(&provider, &signing_keys.public_key_base64())
            .expect_err("missing signature should fail");

        assert_eq!(error.to_string(), "signed envelope missing signature");
    }

    #[test]
    fn signed_envelope_canonical_bytes_omit_none_server_sequence() {
        let bytes = canonical_json_bytes_without_signature(&sample_signed_envelope())
            .expect("canonical bytes");
        let json = String::from_utf8(bytes).expect("utf8");

        assert!(!json.contains("serverSequence"));
    }

    #[test]
    fn encrypted_private_envelope_associated_data_contains_all_authenticated_fields() {
        let json = String::from_utf8(
            sample_private_envelope()
                .associated_data_json()
                .expect("associated data"),
        )
        .expect("utf8");

        assert_eq!(
            json,
            r#"{"counter":8,"messageId":"private-1","messageType":"PRIVATE_HOLE_CARDS_EVENT","protocolVersion":1,"recipientId":"player-1","recipientKeyId":"recipient-key","senderId":"host","serverSequence":9,"sessionEpoch":4,"tableId":"table-1"}"#,
        );
    }

    #[test]
    fn encrypted_private_envelope_from_payload_copies_metadata_exactly() {
        let envelope = EncryptedPrivateEnvelope::from_encrypted_payload(
            EncryptedPayload {
                nonce_base64: "nonce-123".to_string(),
                ciphertext_base64: "ciphertext-456".to_string(),
                recipient_key_id: "recipient-key".to_string(),
            },
            PrivateEnvelopeMetadata {
                sender_id: "host".to_string(),
                table_id: "table-1".to_string(),
                session_epoch: 4,
                counter: 8,
                message_id: "private-1".to_string(),
                server_sequence: 9,
                recipient_id: "player-1".to_string(),
            },
        );

        assert_eq!(
            envelope.message_type,
            ProtocolMessageType::PrivateHoleCardsEvent
        );
        assert_eq!(envelope.recipient_id, "player-1");
        assert_eq!(envelope.recipient_key_id, "recipient-key");
        assert_eq!(envelope.nonce, "nonce-123");
        assert_eq!(envelope.ciphertext, "ciphertext-456");
    }

    #[test]
    fn encrypted_private_envelope_sign_and_verify_track_authenticated_metadata() {
        let provider = DefaultCryptoProvider;
        let signing_keys = provider.generate_signing_keypair();
        let verifying_key = signing_keys.public_key_base64();
        let mut envelope = sample_private_envelope();

        envelope
            .sign(&provider, &signing_keys)
            .expect("sign envelope");

        assert!(envelope.signature.is_some());
        envelope
            .verify(&provider, &verifying_key)
            .expect("signature should verify");

        let mut mutated = envelope.clone();
        mutated.recipient_id = "player-2".to_string();
        assert!(mutated.verify(&provider, &verifying_key).is_err());

        let missing_signature_error = sample_private_envelope()
            .verify(&provider, &verifying_key)
            .expect_err("missing signature should fail");
        assert_eq!(
            missing_signature_error.to_string(),
            "private envelope missing signature"
        );
    }

    #[test]
    fn reconnect_request_serialization_omits_or_keeps_last_known_sequence_as_expected() {
        let without_sequence = serde_json::to_value(ReconnectTournamentRequest {
            player_id: "player-1".to_string(),
            reconnect_token: "token".to_string(),
            last_known_server_seq: None,
        })
        .expect("serialize reconnect request");
        let with_sequence = serde_json::to_value(ReconnectTournamentRequest {
            player_id: "player-1".to_string(),
            reconnect_token: "token".to_string(),
            last_known_server_seq: Some(44),
        })
        .expect("serialize reconnect request");

        assert_eq!(
            without_sequence,
            json!({
                "playerId": "player-1",
                "reconnectToken": "token"
            }),
        );
        assert_eq!(with_sequence.get("lastKnownServerSeq"), Some(&json!(44)));
    }

    #[test]
    fn action_window_opened_serialization_omits_null_raise_bounds_and_keeps_action_strings() {
        let payload = serde_json::to_value(ActionWindowOpened {
            hand_number: 12,
            hand_phase: "TURN".to_string(),
            action_window_id: "window-1".to_string(),
            player_id: "player-1".to_string(),
            seat_index: 3,
            legal_actions: vec![ActionType::Fold, ActionType::Raise, ActionType::AllIn],
            call_amount: 40,
            min_raise_to: None,
            max_raise_to: None,
            deadline_epoch_ms: 123_456,
        })
        .expect("serialize action window");

        assert_eq!(payload.get("minRaiseTo"), None);
        assert_eq!(payload.get("maxRaiseTo"), None);
        assert_eq!(
            payload.get("legalActions"),
            Some(&json!(["FOLD", "RAISE", "ALL_IN"])),
        );
    }

    #[test]
    fn player_action_committed_serialization_preserves_raise_amount_contract() {
        let with_raise = serde_json::to_value(PlayerActionCommitted {
            hand_number: 12,
            seat_index: 3,
            player_id: "player-1".to_string(),
            action_type: ActionType::Raise,
            raise_to_amount: Some(180),
        })
        .expect("serialize action with raise");
        let without_raise = serde_json::to_value(PlayerActionCommitted {
            hand_number: 12,
            seat_index: 3,
            player_id: "player-1".to_string(),
            action_type: ActionType::Call,
            raise_to_amount: None,
        })
        .expect("serialize action without raise");

        assert_eq!(with_raise.get("raiseToAmount"), Some(&json!(180)));
        assert_eq!(
            without_raise.get("raiseToAmount"),
            Some(&serde_json::Value::Null)
        );
    }

    #[test]
    fn protocol_message_type_strings_stay_stable_for_high_risk_variants() {
        assert_eq!(
            serde_json::to_string(&ProtocolMessageType::ReconnectTournamentRequest)
                .expect("serialize message type"),
            r#""RECONNECT_TOURNAMENT_REQUEST""#,
        );
        assert_eq!(
            serde_json::to_string(&ProtocolMessageType::ActionWindowOpenedEvent)
                .expect("serialize message type"),
            r#""ACTION_WINDOW_OPENED_EVENT""#,
        );
        assert_eq!(
            serde_json::to_string(&ProtocolMessageType::PrivateHoleCardsEvent)
                .expect("serialize message type"),
            r#""PRIVATE_HOLE_CARDS_EVENT""#,
        );
    }
}
