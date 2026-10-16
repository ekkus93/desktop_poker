use std::sync::atomic::Ordering;

use base64::Engine as _;

use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    protocol::{
        EncryptedPrivateEnvelope, PrivateEnvelopeMetadata, PrivateHoleCardsEvent,
        ProtocolMessageType, SignedEnvelope, PROTOCOL_VERSION,
    },
};

use super::*;

impl HostServer {
    pub fn broadcast_public_event<TPayload: serde::Serialize>(
        &self,
        message_type: ProtocolMessageType,
        payload: &TPayload,
    ) -> Result<(), NetworkingError> {
        let payload_value = serde_json::to_value(payload)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        let server_sequence = self.server_sequence.fetch_add(1, Ordering::SeqCst) + 1;

        let mut envelope = SignedEnvelope {
            protocol_version: PROTOCOL_VERSION,
            message_type,
            table_id: self.join_payload.table_id.clone(),
            session_epoch: self.join_payload.session_epoch,
            sender_id: "host".to_string(),
            counter: server_sequence,
            message_id: format!("host-{server_sequence}"),
            server_sequence: Some(server_sequence),
            payload: payload_value.clone(),
            signature: None,
        };

        let crypto_provider = DefaultCryptoProvider;
        envelope
            .sign(&crypto_provider, &self.host_signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        append_public_event_log(
            &self.public_events,
            PublicEventLogEntry {
                sequence: server_sequence,
                message_type,
                payload: payload_value,
            },
        )?;

        let failed_clients = client_handles(&self.clients)?
            .into_iter()
            .filter(|handle| write_to_client(handle, &envelope).is_err())
            .collect::<Vec<_>>();
        retire_failed_clients(&self.clients, &failed_clients, |player_id| {
            mark_participant_reconnect_eligible(&self.authoritative_state, player_id)
        })
    }

    pub fn send_private_hole_cards(
        &self,
        recipient_id: &str,
        hole_cards_event: &PrivateHoleCardsEvent,
    ) -> Result<(), NetworkingError> {
        let crypto_provider = DefaultCryptoProvider;

        let recipient = client_handle(&self.clients, recipient_id)?
            .ok_or_else(|| NetworkingError::new("unknown recipient"))?;
        let recipient_encryption_public_key = recipient.encryption_public_key.clone();

        let server_sequence = self.server_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let payload_bytes = serde_json::to_vec(hole_cards_event).map_err(|error| {
            NetworkingError::new(format!("failed to serialize private payload: {error}"))
        })?;

        let host_encryption_keys = self
            .host_encryption_keys
            .lock()
            .map_err(|_| NetworkingError::new("host encryption key lock poisoned"))?;

        let aad_metadata = PrivateEnvelopeMetadata {
            sender_id: "host".to_string(),
            table_id: self.join_payload.table_id.clone(),
            session_epoch: self.join_payload.session_epoch,
            counter: server_sequence,
            message_id: format!("private-{server_sequence}"),
            server_sequence,
            recipient_id: recipient_id.to_string(),
        };

        let aad_bytes = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "messageType": "PRIVATE_HOLE_CARDS_EVENT",
            "tableId": aad_metadata.table_id,
            "sessionEpoch": aad_metadata.session_epoch,
            "senderId": aad_metadata.sender_id,
            "counter": aad_metadata.counter,
            "messageId": aad_metadata.message_id,
            "serverSequence": aad_metadata.server_sequence,
            "recipientId": aad_metadata.recipient_id,
            "recipientKeyId": crate::crypto::key_fingerprint(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(recipient_encryption_public_key.as_bytes())
                    .map_err(|error| NetworkingError::new(format!("invalid recipient public key: {error}")))?,
            )
        });

        let aad = serde_json::to_vec(&aad_bytes)
            .map_err(|error| NetworkingError::new(format!("failed to serialize aad: {error}")))?;
        let encrypted_payload = crypto_provider
            .encrypt(
                &host_encryption_keys,
                &recipient_encryption_public_key,
                &payload_bytes,
                &aad,
            )
            .map_err(|error| NetworkingError::new(error.to_string()))?;
        drop(host_encryption_keys);

        let mut envelope = EncryptedPrivateEnvelope::from_encrypted_payload(
            encrypted_payload,
            PrivateEnvelopeMetadata {
                recipient_id: recipient_id.to_string(),
                ..aad_metadata
            },
        );
        envelope
            .sign(&crypto_provider, &self.host_signing_keys)
            .map_err(|error| NetworkingError::new(error.to_string()))?;

        match write_to_client(&recipient, &envelope) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Demote only if this exact registration is still current; on a
                // poisoned registry fail toward reconnect-eligible.
                let retired =
                    match take_client_if_current(&self.clients, recipient_id, recipient.generation)
                    {
                        Ok(Some(client)) => Some(Some(client)),
                        Ok(None) => None,
                        Err(_) => {
                            update_health(&self.runtime_health, |h| {
                                h.record_client_registry_error();
                            });
                            Some(None)
                        }
                    };
                let Some(retired_client) = retired else {
                    return Err(error);
                };
                if let Err(mark_error) =
                    mark_participant_reconnect_eligible(&self.authoritative_state, recipient_id)
                {
                    update_health(&self.runtime_health, |h| {
                        h.record_reconnect_mark_error(recipient_id, &mark_error);
                    });
                }
                if let Some(client) = retired_client {
                    close_client(&client);
                }
                Err(error)
            }
        }
    }
}
