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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonical_json_bytes, canonical_json_bytes_without_signature};

    #[test]
    fn canonical_json_sorts_keys_and_omits_null_object_fields() {
        let value = json!({
            "z": 1,
            "a": {
                "c": 3,
                "b": 2,
                "drop": null
            },
            "drop": null
        });

        let bytes = canonical_json_bytes(&value).expect("canonical bytes");

        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            r#"{"a":{"b":2,"c":3},"z":1}"#,
        );
    }

    #[test]
    fn canonical_json_preserves_array_order_and_primitives() {
        let value = json!({
            "items": [
                { "b": 2, "a": 1 },
                "alpha",
                7,
                true
            ]
        });

        let bytes = canonical_json_bytes(&value).expect("canonical bytes");

        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            r#"{"items":[{"a":1,"b":2},"alpha",7,true]}"#,
        );
    }

    #[test]
    fn canonical_json_without_signature_removes_only_the_top_level_signature() {
        let value = json!({
            "signature": "outer-signature",
            "payload": {
                "signature": "inner-signature",
                "value": 7
            }
        });

        let bytes = canonical_json_bytes_without_signature(&value)
            .expect("canonical bytes without signature");

        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            r#"{"payload":{"signature":"inner-signature","value":7}}"#,
        );
    }
}
