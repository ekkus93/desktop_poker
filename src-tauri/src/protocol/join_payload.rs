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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use flate2::{write::GzEncoder, Compression};

    use super::{decode_join_payload, encode_join_payload, validate_join_payload};
    use crate::{domain::JoinPayload, protocol::PROTOCOL_VERSION};

    fn sample_payload() -> JoinPayload {
        JoinPayload {
            payload_version: PROTOCOL_VERSION,
            host_address: "192.168.1.40".to_string(),
            host_port: 43_818,
            table_id: "table-join".to_string(),
            session_epoch: 4,
            host_signing_public_key: "host-public-key".to_string(),
            join_token: "join-token".to_string(),
            generated_at_ms: 100,
            table_name: Some("Friday Night".to_string()),
        }
    }

    #[test]
    fn validate_join_payload_rejects_invalid_fields() {
        let invalid_payloads = vec![
            (
                JoinPayload {
                    payload_version: PROTOCOL_VERSION + 1,
                    ..sample_payload()
                },
                "unsupported payloadVersion",
            ),
            (
                JoinPayload {
                    host_address: "   ".to_string(),
                    ..sample_payload()
                },
                "hostAddress must be non-blank",
            ),
            (
                JoinPayload {
                    host_address: "0.0.0.0".to_string(),
                    ..sample_payload()
                },
                "hostAddress must not be 0.0.0.0",
            ),
            (
                JoinPayload {
                    host_port: 0,
                    ..sample_payload()
                },
                "hostPort must be in 1..65535",
            ),
            (
                JoinPayload {
                    host_signing_public_key: " ".to_string(),
                    ..sample_payload()
                },
                "hostSigningPublicKey must be non-blank",
            ),
            (
                JoinPayload {
                    join_token: " ".to_string(),
                    ..sample_payload()
                },
                "joinToken must be non-blank",
            ),
            (
                JoinPayload {
                    table_id: " ".to_string(),
                    ..sample_payload()
                },
                "tableId must be non-blank",
            ),
            (
                JoinPayload {
                    generated_at_ms: 0,
                    ..sample_payload()
                },
                "generatedAtMs must be greater than zero",
            ),
        ];

        for (payload, expected_message) in invalid_payloads {
            assert_eq!(
                validate_join_payload(&payload)
                    .expect_err("payload should be rejected")
                    .to_string(),
                expected_message,
            );
        }
    }

    #[test]
    fn compact_join_payload_round_trips_and_keeps_the_prefix() {
        let payload = sample_payload();

        let encoded = encode_join_payload(&payload).expect("payload should encode");

        assert!(encoded.starts_with("pkr1_"));
        assert_eq!(
            decode_join_payload(&encoded).expect("payload should decode"),
            payload
        );
    }

    #[test]
    fn legacy_raw_json_join_payloads_still_decode() {
        let payload = sample_payload();
        let raw_json = serde_json::to_string(&payload).expect("payload JSON");

        assert_eq!(
            decode_join_payload(&raw_json).expect("legacy payload should decode"),
            payload
        );
    }

    #[test]
    fn invalid_base64_compact_payloads_return_the_expected_error() {
        let error =
            decode_join_payload("pkr1_***invalid***").expect_err("base64 payload should fail");

        assert!(error.to_string().contains("invalid compact join payload"));
    }

    #[test]
    fn invalid_gzip_compact_payloads_return_the_expected_error() {
        let encoded = format!("pkr1_{}", URL_SAFE_NO_PAD.encode(b"not-gzip"));

        let error = decode_join_payload(&encoded).expect_err("gzip payload should fail");

        assert!(error.to_string().contains("gzip decode failed"));
    }

    #[test]
    fn invalid_decoded_json_returns_the_expected_error() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(br#"{"broken":true}"#)
            .expect("write invalid JSON body");
        let compressed = encoder.finish().expect("finish gzip");
        let encoded = format!("pkr1_{}", URL_SAFE_NO_PAD.encode(compressed));

        let error = decode_join_payload(&encoded).expect_err("JSON payload should fail");

        assert!(error.to_string().contains("invalid join payload JSON"));
    }

    #[test]
    fn decoded_payloads_still_fail_final_validation_when_semantically_invalid() {
        let invalid_payload = JoinPayload {
            host_port: 0,
            ..sample_payload()
        };
        let canonical_json = serde_json::to_vec(&invalid_payload).expect("payload bytes");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&canonical_json)
            .expect("write payload bytes");
        let compressed = encoder.finish().expect("finish gzip");
        let encoded = format!("pkr1_{}", URL_SAFE_NO_PAD.encode(compressed));

        let error = decode_join_payload(&encoded).expect_err("invalid payload should fail");

        assert_eq!(error.to_string(), "hostPort must be in 1..65535");
    }
}
