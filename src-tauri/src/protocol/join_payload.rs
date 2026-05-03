use std::io::{Read, Write};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};

use crate::domain::JoinPayload;

use super::{canonical_json_bytes, ProtocolError, JOIN_PAYLOAD_PREFIX, PROTOCOL_VERSION};

pub fn validate_join_payload(payload: &JoinPayload) -> Result<(), ProtocolError> {
    if payload.payload_version != PROTOCOL_VERSION {
        return Err(ProtocolError::new("unsupported payloadVersion"));
    }

    if payload.host_address.trim().is_empty() {
        return Err(ProtocolError::new("hostAddress must be non-blank"));
    }

    if payload.host_port == 0 {
        return Err(ProtocolError::new("hostPort must be in 1..65535"));
    }

    if payload.host_address == "0.0.0.0" {
        return Err(ProtocolError::new("hostAddress must not be 0.0.0.0"));
    }

    if payload.host_signing_public_key.trim().is_empty() {
        return Err(ProtocolError::new("hostSigningPublicKey must be non-blank"));
    }

    if payload.join_token.trim().is_empty() {
        return Err(ProtocolError::new("joinToken must be non-blank"));
    }

    if payload.table_id.trim().is_empty() {
        return Err(ProtocolError::new("tableId must be non-blank"));
    }

    if payload.generated_at_ms == 0 {
        return Err(ProtocolError::new(
            "generatedAtMs must be greater than zero",
        ));
    }

    Ok(())
}

pub fn encode_join_payload(payload: &JoinPayload) -> Result<String, ProtocolError> {
    validate_join_payload(payload)?;

    let canonical_bytes = canonical_json_bytes(payload)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&canonical_bytes)
        .map_err(|error| ProtocolError::new(format!("gzip encoding failed: {error}")))?;

    let compressed = encoder
        .finish()
        .map_err(|error| ProtocolError::new(format!("gzip finalize failed: {error}")))?;

    Ok(format!(
        "{JOIN_PAYLOAD_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(compressed)
    ))
}

pub fn decode_join_payload(encoded: &str) -> Result<JoinPayload, ProtocolError> {
    let payload = if let Some(compact_payload) = encoded.strip_prefix(JOIN_PAYLOAD_PREFIX) {
        let compressed = URL_SAFE_NO_PAD
            .decode(compact_payload.as_bytes())
            .map_err(|error| {
                ProtocolError::new(format!("invalid compact join payload: {error}"))
            })?;
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .map_err(|error| ProtocolError::new(format!("gzip decode failed: {error}")))?;

        serde_json::from_str::<JoinPayload>(&json)
            .map_err(|error| ProtocolError::new(format!("invalid join payload JSON: {error}")))?
    } else {
        serde_json::from_str::<JoinPayload>(encoded).map_err(|error| {
            ProtocolError::new(format!("invalid legacy join payload JSON: {error}"))
        })?
    };

    validate_join_payload(&payload)?;
    Ok(payload)
}
