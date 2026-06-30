use std::sync::{atomic::Ordering, Arc};

use base64::Engine as _;

use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    networking::write_json_frame,
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

        let failed_clients = {
            let connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            let mut failed_clients = Vec::new();

            for (player_id, client) in connected_clients.iter() {
                let write_result = client
                    .stream
                    .lock()
                    .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
                    .try_clone()
                    .map_err(|error| {
                        NetworkingError::new(format!("failed to clone client stream: {error}"))
                    })
                    .and_then(|mut stream| write_json_frame(&mut stream, &envelope));

                if write_result.is_err() {
                    failed_clients.push(player_id.clone());
                }
            }

            failed_clients
        };

        if !failed_clients.is_empty() {
            let mut connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            for player_id in failed_clients {
                connected_clients.remove(&player_id);
                mark_participant_reconnect_eligible(&self.authoritative_state, &player_id)?;
            }
        }

        Ok(())
    }

    pub fn send_private_hole_cards(
        &self,
        recipient_id: &str,
        hole_cards_event: &PrivateHoleCardsEvent,
    ) -> Result<(), NetworkingError> {
        let crypto_provider = DefaultCryptoProvider;

        let (client_stream, recipient_encryption_public_key) = {
            let connected_clients = self
                .clients
                .lock()
                .map_err(|_| NetworkingError::new("client registry lock poisoned"))?;
            let client = connected_clients
                .get(recipient_id)
                .ok_or_else(|| NetworkingError::new("unknown recipient"))?;

            (
                Arc::clone(&client.stream),
                client.encryption_public_key.clone(),
            )
        };

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

        let mut stream = client_stream
            .lock()
            .map_err(|_| NetworkingError::new("client stream lock poisoned"))?
            .try_clone()
            .map_err(|error| {
                NetworkingError::new(format!("failed to clone client stream: {error}"))
            })?;

        match write_json_frame(&mut stream, &envelope) {
            Ok(()) => Ok(()),
            Err(error) => {
                match self.clients.lock() {
                    Ok(mut connected_clients) => {
                        connected_clients.remove(recipient_id);
                    }
                    Err(_) => {
                        update_health(&self.runtime_health, |h| {
                            h.record_client_registry_error();
                        });
                    }
                }
                if let Err(mark_error) =
                    mark_participant_reconnect_eligible(&self.authoritative_state, recipient_id)
                {
                    update_health(&self.runtime_health, |h| {
                        h.record_reconnect_mark_error(recipient_id, &mark_error);
                    });
                }
                Err(error)
            }
        }
    }
}
