use std::io::{Read, Write};
use std::net::TcpStream;

use serde::{de::DeserializeOwned, Serialize};

use super::NetworkingError;

/// Maximum accepted JSON frame payload size.
///
/// The length prefix is controlled by the remote peer, so this limit must be
/// checked before allocating the body buffer.
pub const MAX_FRAME_PAYLOAD_BYTES: usize = 1_048_576;

pub fn write_json_frame<T: Serialize>(
    stream: &mut TcpStream,
    payload: &T,
) -> Result<(), NetworkingError> {
    write_json_frame_to_writer(stream, payload)
}

pub fn read_json_frame<T: DeserializeOwned>(stream: &mut TcpStream) -> Result<T, NetworkingError> {
    read_json_frame_from_reader(stream)
}

fn write_json_frame_to_writer<T: Serialize, W: Write>(
    writer: &mut W,
    payload: &T,
) -> Result<(), NetworkingError> {
    let bytes =
        serde_json::to_vec(payload).map_err(|error| NetworkingError::new(error.to_string()))?;

    write_frame_bytes(writer, &bytes, bytes.len() as u64)
}

fn write_frame_bytes<W: Write>(
    writer: &mut W,
    payload_bytes: &[u8],
    payload_len: u64,
) -> Result<(), NetworkingError> {
    let length = u32::try_from(payload_len)
        .map_err(|_| NetworkingError::new("frame payload exceeds u32 length"))?;

    writer
        .write_all(&length.to_be_bytes())
        .map_err(|error| NetworkingError::new(format!("failed to write frame length: {error}")))?;
    writer
        .write_all(payload_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to write frame body: {error}")))?;
    writer
        .flush()
        .map_err(|error| NetworkingError::new(format!("failed to flush frame: {error}")))?;

    Ok(())
}

fn read_json_frame_from_reader<T: DeserializeOwned, R: Read>(
    reader: &mut R,
) -> Result<T, NetworkingError> {
    let mut length_bytes = [0_u8; 4];
    reader
        .read_exact(&mut length_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame length: {error}")))?;
    let length = u32::from_be_bytes(length_bytes) as usize;

    if length > MAX_FRAME_PAYLOAD_BYTES {
        return Err(NetworkingError::new(format!(
            "frame payload exceeds maximum allowed size: {length} > {MAX_FRAME_PAYLOAD_BYTES}"
        )));
    }

    let mut payload_bytes = vec![0_u8; length];
    reader
        .read_exact(&mut payload_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame body: {error}")))?;

    serde_json::from_slice(&payload_bytes)
        .map_err(|error| NetworkingError::new(format!("invalid frame JSON: {error}")))
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor};

    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::{
        read_json_frame_from_reader, write_frame_bytes, write_json_frame_to_writer,
        MAX_FRAME_PAYLOAD_BYTES,
    };

    #[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
    struct SamplePayload {
        value: String,
    }

    #[derive(Debug)]
    enum FailingWriterMode {
        Length,
        Body,
        Flush,
    }

    #[derive(Debug)]
    struct FailingWriter {
        mode: FailingWriterMode,
        writes: usize,
        bytes: Vec<u8>,
    }

    impl FailingWriter {
        fn new(mode: FailingWriterMode) -> Self {
            Self {
                mode,
                writes: 0,
                bytes: Vec::new(),
            }
        }
    }

    impl io::Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            let should_fail = matches!(self.mode, FailingWriterMode::Length)
                || (matches!(self.mode, FailingWriterMode::Body) && self.writes > 1);

            if should_fail {
                return Err(io::Error::other("write failed"));
            }

            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if matches!(self.mode, FailingWriterMode::Flush) {
                return Err(io::Error::other("flush failed"));
            }

            Ok(())
        }
    }

    #[test]
    fn write_json_frame_writes_a_big_endian_length_prefix_and_json_body() {
        let payload = SamplePayload {
            value: "alpha".to_string(),
        };
        let mut writer = Vec::new();

        write_json_frame_to_writer(&mut writer, &payload).expect("frame should write");

        let body = serde_json::to_vec(&payload).expect("payload bytes");
        assert_eq!(&writer[..4], &(body.len() as u32).to_be_bytes());
        assert_eq!(&writer[4..], body.as_slice());
    }

    #[test]
    fn read_json_frame_decodes_a_valid_frame() {
        let payload = SamplePayload {
            value: "alpha".to_string(),
        };
        let body = serde_json::to_vec(&payload).expect("payload bytes");
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&body);

        let decoded: SamplePayload =
            read_json_frame_from_reader(&mut Cursor::new(bytes)).expect("frame should decode");

        assert_eq!(decoded, payload);
    }

    #[test]
    fn frames_round_trip_through_write_then_read() {
        let payload = SamplePayload {
            value: "beta".to_string(),
        };
        let mut writer = Vec::new();

        write_json_frame_to_writer(&mut writer, &payload).expect("frame should write");
        let decoded: SamplePayload =
            read_json_frame_from_reader(&mut Cursor::new(writer)).expect("frame should decode");

        assert_eq!(decoded, payload);
    }

    #[test]
    fn read_json_frame_rejects_payload_larger_than_max_before_allocation() {
        let advertised_length = u32::try_from(MAX_FRAME_PAYLOAD_BYTES + 1)
            .expect("configured frame limit should fit in u32");
        let bytes = advertised_length.to_be_bytes().to_vec();

        let error = read_json_frame_from_reader::<SamplePayload, _>(&mut Cursor::new(bytes))
            .expect_err("oversized frame should fail before reading a body");

        assert_eq!(
            error.to_string(),
            format!(
                "frame payload exceeds maximum allowed size: {} > {}",
                MAX_FRAME_PAYLOAD_BYTES + 1,
                MAX_FRAME_PAYLOAD_BYTES
            )
        );
    }

    #[test]
    fn read_json_frame_reports_truncated_length_prefixes() {
        let error = read_json_frame_from_reader::<SamplePayload, _>(&mut Cursor::new(vec![0, 0]))
            .expect_err("length read should fail");

        assert!(error.to_string().contains("failed to read frame length"));
    }

    #[test]
    fn read_json_frame_reports_truncated_bodies() {
        let mut bytes = 10_u32.to_be_bytes().to_vec();
        bytes.extend_from_slice(br#"{"value":"x"}"#);
        bytes.truncate(8);

        let error = read_json_frame_from_reader::<SamplePayload, _>(&mut Cursor::new(bytes))
            .expect_err("body read should fail");

        assert!(error.to_string().contains("failed to read frame body"));
    }

    #[test]
    fn read_json_frame_reports_invalid_json_syntax() {
        let body = b"{not-json}".to_vec();
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&body);

        let error = read_json_frame_from_reader::<SamplePayload, _>(&mut Cursor::new(bytes))
            .expect_err("invalid JSON should fail");

        assert!(error.to_string().contains("invalid frame JSON"));
    }

    #[test]
    fn read_json_frame_reports_type_mismatches_as_invalid_json() {
        let body = serde_json::to_vec(&json!({ "other": 7 })).expect("payload bytes");
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&body);

        let error = read_json_frame_from_reader::<SamplePayload, _>(&mut Cursor::new(bytes))
            .expect_err("type mismatch should fail");

        assert!(error.to_string().contains("invalid frame JSON"));
    }

    #[test]
    fn write_json_frame_reports_length_write_failures() {
        let payload = SamplePayload {
            value: "alpha".to_string(),
        };
        let mut writer = FailingWriter::new(FailingWriterMode::Length);

        let error = write_json_frame_to_writer(&mut writer, &payload)
            .expect_err("length write should fail");

        assert!(error.to_string().contains("failed to write frame length"));
    }

    #[test]
    fn write_json_frame_reports_body_write_failures() {
        let payload = SamplePayload {
            value: "alpha".to_string(),
        };
        let mut writer = FailingWriter::new(FailingWriterMode::Body);

        let error =
            write_json_frame_to_writer(&mut writer, &payload).expect_err("body write should fail");

        assert!(error.to_string().contains("failed to write frame body"));
    }

    #[test]
    fn write_json_frame_reports_flush_failures() {
        let payload = SamplePayload {
            value: "alpha".to_string(),
        };
        let mut writer = FailingWriter::new(FailingWriterMode::Flush);

        let error =
            write_json_frame_to_writer(&mut writer, &payload).expect_err("flush should fail");

        assert!(error.to_string().contains("failed to flush frame"));
    }

    #[test]
    fn write_frame_bytes_rejects_payloads_larger_than_u32() {
        let mut writer = Vec::new();

        let error = write_frame_bytes(&mut writer, &[], u64::from(u32::MAX) + 1)
            .expect_err("oversized payload should fail");

        assert_eq!(error.to_string(), "frame payload exceeds u32 length");
    }
}
