use std::io::{Read, Write};
use std::net::TcpStream;

use serde::{de::DeserializeOwned, Serialize};

use super::NetworkingError;

pub fn write_json_frame<T: Serialize>(
    stream: &mut TcpStream,
    payload: &T,
) -> Result<(), NetworkingError> {
    let bytes =
        serde_json::to_vec(payload).map_err(|error| NetworkingError::new(error.to_string()))?;
    let length = u32::try_from(bytes.len())
        .map_err(|_| NetworkingError::new("frame payload exceeds u32 length"))?;

    stream
        .write_all(&length.to_be_bytes())
        .map_err(|error| NetworkingError::new(format!("failed to write frame length: {error}")))?;
    stream
        .write_all(&bytes)
        .map_err(|error| NetworkingError::new(format!("failed to write frame body: {error}")))?;
    stream
        .flush()
        .map_err(|error| NetworkingError::new(format!("failed to flush frame: {error}")))?;

    Ok(())
}

pub fn read_json_frame<T: DeserializeOwned>(stream: &mut TcpStream) -> Result<T, NetworkingError> {
    let mut length_bytes = [0_u8; 4];
    stream
        .read_exact(&mut length_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame length: {error}")))?;
    let length = u32::from_be_bytes(length_bytes) as usize;

    let mut payload_bytes = vec![0_u8; length];
    stream
        .read_exact(&mut payload_bytes)
        .map_err(|error| NetworkingError::new(format!("failed to read frame body: {error}")))?;

    serde_json::from_slice(&payload_bytes)
        .map_err(|error| NetworkingError::new(format!("invalid frame JSON: {error}")))
}
