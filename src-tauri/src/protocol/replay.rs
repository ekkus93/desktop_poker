use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::{EncryptedPrivateEnvelope, ProtocolError, SignedEnvelope};

#[derive(Clone, Debug)]
pub struct ReplayProtector {
    table_id: String,
    session_epoch: u64,
    last_counter_by_sender: HashMap<String, u64>,
    seen_message_ids: HashSet<String>,
}

impl ReplayProtector {
    #[must_use]
    pub fn new(table_id: impl Into<String>, session_epoch: u64) -> Self {
        Self {
            table_id: table_id.into(),
            session_epoch,
            last_counter_by_sender: HashMap::new(),
            seen_message_ids: HashSet::new(),
        }
    }

    pub fn validate_signed<TPayload: Serialize>(
        &mut self,
        envelope: &SignedEnvelope<TPayload>,
    ) -> Result<(), ProtocolError> {
        self.validate_common(
            &envelope.table_id,
            envelope.session_epoch,
            &envelope.sender_id,
            envelope.counter,
            &envelope.message_id,
        )
    }

    pub fn validate_private(
        &mut self,
        envelope: &EncryptedPrivateEnvelope,
    ) -> Result<(), ProtocolError> {
        self.validate_common(
            &envelope.table_id,
            envelope.session_epoch,
            &envelope.sender_id,
            envelope.counter,
            &envelope.message_id,
        )
    }

    fn validate_common(
        &mut self,
        table_id: &str,
        session_epoch: u64,
        sender_id: &str,
        counter: u64,
        message_id: &str,
    ) -> Result<(), ProtocolError> {
        if table_id != self.table_id {
            return Err(ProtocolError::new("mismatched table identifier"));
        }

        if session_epoch != self.session_epoch {
            return Err(ProtocolError::new("stale or mismatched session epoch"));
        }

        if self.seen_message_ids.contains(message_id) {
            return Err(ProtocolError::new("duplicate messageId rejected"));
        }

        if self
            .last_counter_by_sender
            .get(sender_id)
            .is_some_and(|last_counter| counter <= *last_counter)
        {
            return Err(ProtocolError::new("stale counter rejected"));
        }

        self.last_counter_by_sender
            .insert(sender_id.to_string(), counter);
        self.seen_message_ids.insert(message_id.to_string());

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerSequenceTracker {
    last_seen_sequence: Option<u64>,
}

impl ServerSequenceTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, next_sequence: u64) -> Result<(), ProtocolError> {
        if let Some(last_seen_sequence) = self.last_seen_sequence {
            if next_sequence != last_seen_sequence + 1 {
                return Err(ProtocolError::new("server sequence gap detected"));
            }
        }

        self.last_seen_sequence = Some(next_sequence);
        Ok(())
    }

    #[must_use]
    pub fn last_seen(&self) -> Option<u64> {
        self.last_seen_sequence
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::protocol::{
        canonical_json_bytes, canonical_json_bytes_without_signature, decode_join_payload,
        encode_join_payload, validate_join_payload, ActionRejectedEvent, EncryptedPrivateEnvelope,
        JoinTournamentRequest, JsonSignedEnvelope, PrivateEnvelopeMetadata, PrivateHoleCardsEvent,
        ProtocolMessageType, ReplayProtector, ServerSequenceTracker, SignedEnvelope,
        PROTOCOL_VERSION,
    };
    use crate::{
        crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
        domain::JoinPayload,
    };

    #[test]
    fn canonical_json_orders_fields_and_omits_nulls() {
        let envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::ReadyStateRequest,
            table_id: "table-1".to_string(),
            session_epoch: 7,
            sender_id: "player-a".to_string(),
            counter: 1,
            message_id: "msg-1".to_string(),
            server_sequence: None,
            payload: json!({
                "zebra": 1,
                "alpha": {"delta": true, "beta": 2, "gamma": null}
            }),
            signature: None,
        };

        let bytes = canonical_json_bytes_without_signature(&envelope).expect("canonical bytes");

        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            "{\"counter\":1,\"messageId\":\"msg-1\",\"messageType\":\"READY_STATE_REQUEST\",\"payload\":{\"alpha\":{\"beta\":2,\"delta\":true},\"zebra\":1},\"protocolVersion\":1,\"senderId\":\"player-a\",\"sessionEpoch\":7,\"tableId\":\"table-1\"}"
        );
    }

    #[test]
    fn join_payload_compact_codec_round_trips_and_validates() {
        let payload = JoinPayload {
            payload_version: 1,
            host_address: "192.168.1.44".to_string(),
            host_port: 43_818,
            table_id: "table-1".to_string(),
            session_epoch: 9,
            host_signing_public_key: "host-key".to_string(),
            join_token: "token-123".to_string(),
            generated_at_ms: 123_456,
            table_name: Some("Friday".to_string()),
        };

        validate_join_payload(&payload).expect("payload should validate");
        let encoded = encode_join_payload(&payload).expect("payload should encode");
        let decoded = decode_join_payload(&encoded).expect("payload should decode");

        assert_eq!(decoded, payload);
    }

    #[test]
    fn signed_envelope_signs_and_verifies() {
        let provider = DefaultCryptoProvider;
        let signing_keys = provider.generate_signing_keypair();

        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::JoinTournamentRequest,
            table_id: "table-1".to_string(),
            session_epoch: 7,
            sender_id: "player-a".to_string(),
            counter: 1,
            message_id: "msg-1".to_string(),
            server_sequence: None,
            payload: JoinTournamentRequest {
                display_name: "Alice".to_string(),
                join_token: "token".to_string(),
                signing_public_key: signing_keys.public_key_base64(),
                encryption_public_key: "enc-key".to_string(),
            },
            signature: None,
        };

        envelope
            .sign(&provider, &signing_keys)
            .expect("signing should succeed");

        envelope
            .verify(&provider, &signing_keys.public_key_base64())
            .expect("verification should succeed");
    }

    #[test]
    fn encrypted_private_payload_round_trips() {
        let provider = DefaultCryptoProvider;
        let sender_encryption_keys = provider.generate_encryption_keypair();
        let sender_signing_keys = provider.generate_signing_keypair();
        let recipient_encryption_keys = provider.generate_encryption_keypair();
        let payload = PrivateHoleCardsEvent {
            recipient_player_id: "player-b".to_string(),
            hole_cards: Vec::new(),
        };

        let mut envelope = EncryptedPrivateEnvelope::from_encrypted_payload(
            provider
                .encrypt(
                    &sender_encryption_keys,
                    &recipient_encryption_keys.public_key_base64(),
                    canonical_json_bytes(&payload)
                        .expect("payload bytes")
                        .as_slice(),
                    &canonical_json_bytes(&json!({
                        "protocolVersion": 1,
                        "messageType": "PRIVATE_HOLE_CARDS_EVENT",
                        "tableId": "table-1",
                        "sessionEpoch": 7,
                        "senderId": "player-a",
                        "counter": 2,
                        "messageId": "msg-2",
                        "serverSequence": 1,
                        "recipientId": "player-b",
                        "recipientKeyId": recipient_encryption_keys.key_id()
                    }))
                    .expect("aad"),
                )
                .expect("encryption should succeed"),
            PrivateEnvelopeMetadata {
                sender_id: "player-a".to_string(),
                table_id: "table-1".to_string(),
                session_epoch: 7,
                counter: 2,
                message_id: "msg-2".to_string(),
                server_sequence: 1,
                recipient_id: "player-b".to_string(),
            },
        );

        envelope
            .sign(&provider, &sender_signing_keys)
            .expect("signing should succeed");
        envelope
            .verify(&provider, &sender_signing_keys.public_key_base64())
            .expect("verification should succeed");

        let decrypted = provider
            .decrypt(
                &recipient_encryption_keys,
                &sender_encryption_keys.public_key_base64(),
                &crate::crypto::EncryptedPayload {
                    nonce_base64: envelope.nonce.clone(),
                    ciphertext_base64: envelope.ciphertext.clone(),
                    recipient_key_id: envelope.recipient_key_id.clone(),
                },
                envelope.associated_data_json().expect("aad").as_slice(),
            )
            .expect("decryption should succeed");

        assert_eq!(
            decrypted,
            canonical_json_bytes(&payload).expect("payload bytes")
        );
    }

    #[test]
    fn replay_and_sequence_guards_reject_duplicates_and_gaps() {
        let mut replay = ReplayProtector::new("table-1", 7);
        let envelope = JsonSignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type: ProtocolMessageType::ActionRejectedEvent,
            table_id: "table-1".to_string(),
            session_epoch: 7,
            sender_id: "host".to_string(),
            counter: 4,
            message_id: "msg-4".to_string(),
            server_sequence: Some(3),
            payload: serde_json::to_value(ActionRejectedEvent {
                seat_index: 1,
                action_type: crate::domain::ActionType::Fold,
                reason: "stale".to_string(),
            })
            .expect("payload value"),
            signature: None,
        };

        replay.validate_signed(&envelope).expect("first event");
        assert!(replay.validate_signed(&envelope).is_err());

        let mut tracker = ServerSequenceTracker::new();
        tracker.observe(1).expect("sequence 1");
        assert!(tracker.observe(3).is_err());
    }
}
