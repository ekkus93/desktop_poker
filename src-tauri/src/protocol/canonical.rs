use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use super::ProtocolError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalJsonFixture {
    pub name: &'static str,
    pub expected_json: &'static str,
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let raw_value =
        serde_json::to_value(value).map_err(|error| ProtocolError::new(error.to_string()))?;

    canonical_json_bytes_from_value(raw_value)
}

pub fn canonical_json_bytes_without_signature<T: Serialize>(
    value: &T,
) -> Result<Vec<u8>, ProtocolError> {
    let mut raw_value =
        serde_json::to_value(value).map_err(|error| ProtocolError::new(error.to_string()))?;

    if let Value::Object(object) = &mut raw_value {
        object.remove("signature");
    }

    canonical_json_bytes_from_value(raw_value)
}

fn canonical_json_bytes_from_value(value: Value) -> Result<Vec<u8>, ProtocolError> {
    let canonical = normalize_value(value);
    serde_json::to_vec(&canonical).map_err(|error| ProtocolError::new(error.to_string()))
}

fn normalize_value(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut sorted = BTreeMap::new();

            for (key, value) in object {
                if value.is_null() {
                    continue;
                }

                sorted.insert(key, normalize_value(value));
            }

            let mut normalized = Map::new();
            for (key, value) in sorted {
                normalized.insert(key, value);
            }

            Value::Object(normalized)
        }
        Value::Array(values) => {
            Value::Array(values.into_iter().map(normalize_value).collect::<Vec<_>>())
        }
        primitive => primitive,
    }
}
