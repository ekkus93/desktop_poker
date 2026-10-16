use std::io::{Read, Write};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};

use crate::domain::JoinPayload;

use super::{canonical_json_bytes, ProtocolError, JOIN_PAYLOAD_PREFIX, PROTOCOL_VERSION};

const MAX_COMPACT_ENCODED_BYTES: usize = 16 * 1024;
const MAX_COMPACT_COMPRESSED_BYTES: usize = 12 * 1024;
const MAX_JOIN_JSON_BYTES: usize = 64 * 1024;

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
        if encoded.len() > MAX_COMPACT_ENCODED_BYTES {
            return Err(ProtocolError::new(
                "compact join payload exceeds size limit",
            ));
        }
        let compressed = URL_SAFE_NO_PAD
            .decode(compact_payload.as_bytes())
            .map_err(|error| {
                ProtocolError::new(format!("invalid compact join payload: {error}"))
            })?;
        if compressed.len() > MAX_COMPACT_COMPRESSED_BYTES {
            return Err(ProtocolError::new(
                "compressed join payload exceeds size limit",
            ));
        }

        // Apply the bound *while* decompressing. A length check after
        // read_to_string would already have allocated the expanded input.
        let decoder = GzDecoder::new(compressed.as_slice());
        let mut json_bytes = Vec::new();
        decoder
            .take((MAX_JOIN_JSON_BYTES + 1) as u64)
            .read_to_end(&mut json_bytes)
            .map_err(|error| ProtocolError::new(format!("gzip decode failed: {error}")))?;
        if json_bytes.len() > MAX_JOIN_JSON_BYTES {
            return Err(ProtocolError::new(
                "decompressed join payload exceeds size limit",
            ));
        }
        let json = String::from_utf8(json_bytes)
            .map_err(|error| ProtocolError::new(format!("gzip decode failed: {error}")))?;

        serde_json::from_str::<JoinPayload>(&json)
            .map_err(|error| ProtocolError::new(format!("invalid join payload JSON: {error}")))?
    } else {
        if encoded.len() > MAX_JOIN_JSON_BYTES {
            return Err(ProtocolError::new("legacy join payload exceeds size limit"));
        }
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
    fn oversized_compact_input_is_rejected_before_base64_decode() {
        let oversized = format!("pkr1_{}", "A".repeat(super::MAX_COMPACT_ENCODED_BYTES + 1));
        let error = decode_join_payload(&oversized).expect_err("oversized compact input");
        assert!(error.to_string().contains("size limit"));
    }

    #[test]
    fn compact_prefix_counts_toward_the_total_encoded_size_limit() {
        // The prefix is part of the untrusted invitation and its byte budget.
        let oversized = format!(
            "pkr1_{}",
            "A".repeat(super::MAX_COMPACT_ENCODED_BYTES - "pkr1_".len() + 1)
        );
        assert_eq!(oversized.len(), super::MAX_COMPACT_ENCODED_BYTES + 1);
        let error = decode_join_payload(&oversized).expect_err("oversized invitation");
        assert!(error.to_string().contains("size limit"));
    }

    #[test]
    fn oversized_compressed_payload_is_rejected_before_decompression() {
        let compressed = vec![0u8; super::MAX_COMPACT_COMPRESSED_BYTES + 1];
        let encoded = format!("pkr1_{}", URL_SAFE_NO_PAD.encode(compressed));
        let error = decode_join_payload(&encoded).expect_err("oversized compressed input");
        assert!(error.to_string().contains("size limit"));
    }

    #[test]
    fn gzip_expansion_bomb_is_bounded_during_decompression() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all("x".repeat(super::MAX_JOIN_JSON_BYTES + 1).as_bytes())
            .expect("write expansion fixture");
        let encoded = format!(
            "pkr1_{}",
            URL_SAFE_NO_PAD.encode(encoder.finish().expect("gzip fixture"))
        );
        let error = decode_join_payload(&encoded).expect_err("expanded payload too large");
        assert!(error
            .to_string()
            .contains("decompressed join payload exceeds size limit"));
    }

    #[test]
    fn decompressed_json_accepts_the_exact_limit_and_rejects_invalid_utf8() {
        let mut json = serde_json::to_vec(&sample_payload()).expect("payload JSON");
        json.resize(super::MAX_JOIN_JSON_BYTES, b' ');
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&json).expect("write exact-limit fixture");
        let encoded = format!(
            "pkr1_{}",
            URL_SAFE_NO_PAD.encode(encoder.finish().expect("finish gzip"))
        );
        assert_eq!(decode_join_payload(&encoded).unwrap(), sample_payload());

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&[0xff, 0xfe])
            .expect("write invalid UTF-8 fixture");
        let invalid_utf8 = format!(
            "pkr1_{}",
            URL_SAFE_NO_PAD.encode(encoder.finish().expect("finish gzip"))
        );
        let error = decode_join_payload(&invalid_utf8).expect_err("invalid UTF-8");
        assert!(error.to_string().contains("gzip decode failed"));
    }

    #[test]
    fn legacy_json_accepts_the_exact_limit_and_rejects_one_byte_over() {
        let mut json = serde_json::to_string(&sample_payload()).expect("payload JSON");
        json.push_str(&" ".repeat(super::MAX_JOIN_JSON_BYTES - json.len()));
        assert_eq!(json.len(), super::MAX_JOIN_JSON_BYTES);
        assert_eq!(decode_join_payload(&json).unwrap(), sample_payload());

        json.push(' ');
        let error = decode_join_payload(&json).expect_err("legacy input one byte over");
        assert!(error
            .to_string()
            .contains("legacy join payload exceeds size limit"));
    }

    #[test]
    fn oversized_legacy_json_is_rejected_before_parsing() {
        let oversized = format!(
            "{{\"untrusted\":\"{}\"}}",
            "x".repeat(super::MAX_JOIN_JSON_BYTES)
        );
        let error = decode_join_payload(&oversized).expect_err("oversized legacy JSON");
        assert!(error
            .to_string()
            .contains("legacy join payload exceeds size limit"));
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
