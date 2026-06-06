//! Definition canonicalization (RFC-8785 JCS) and the SHA-256 binding hash (§9.2).
//!
//! The content hash is computed over the canonical bytes with `header.contentHash`
//! cleared (self-exclusion); the carriage hash is the first 8 digest bytes in digest order.

use crate::model::DefinitionFile;
use serde::Serialize;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

/// Full and truncated definition hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionHash {
    /// Lowercase 64-character SHA-256 hex digest.
    pub content_hash: String,
    /// First 8 digest bytes in digest order.
    pub carriage_hash: [u8; 8],
}

/// Errors from definition canonicalization and hashing.
#[derive(Debug)]
pub enum DefinitionError {
    /// The input was not valid definition JSON (parse or duplicate-key/float guard failure).
    Json(String),
    /// Canonicalization (RFC-8785 JCS) of the definition failed.
    Canonical(String),
}

impl fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(f, "invalid definition JSON: {message}"),
            Self::Canonical(message) => write!(f, "canonicalization failed: {message}"),
        }
    }
}

impl std::error::Error for DefinitionError {}

/// Serializes a typed value using RFC-8785 canonical JSON.
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, DefinitionError> {
    serde_jcs::to_vec(value).map_err(|err| DefinitionError::Canonical(err.to_string()))
}

/// Parses raw JSON with duplicate-key and no-float guardrails, then canonicalizes it.
pub fn canonical_json_bytes_from_str(input: &str) -> Result<Vec<u8>, DefinitionError> {
    let value = parse_json_value(input)?;
    canonical_json_bytes(&value)
}

/// Parses and canonicalizes raw JSON as UTF-8 text.
pub fn canonical_json_string_from_str(input: &str) -> Result<String, DefinitionError> {
    let bytes = canonical_json_bytes_from_str(input)?;
    String::from_utf8(bytes).map_err(|err| DefinitionError::Canonical(err.to_string()))
}

/// Computes the §9.2 content hash over canonical bytes with `header.contentHash=""`.
pub fn compute_content_hash(
    definition: &DefinitionFile,
) -> Result<DefinitionHash, DefinitionError> {
    let canonical = canonical_definition_bytes_for_hash(definition)?;
    let digest = Sha256::digest(&canonical);
    let mut carriage_hash = [0u8; 8];
    carriage_hash.copy_from_slice(&digest[..8]);

    Ok(DefinitionHash {
        content_hash: lower_hex(&digest),
        carriage_hash,
    })
}

/// Canonical definition bytes used as the §9.2 hash preimage.
pub fn canonical_definition_bytes_for_hash(
    definition: &DefinitionFile,
) -> Result<Vec<u8>, DefinitionError> {
    let mut for_hash = definition.clone();
    for_hash.header.content_hash.clear();
    canonical_json_bytes(&for_hash)
}

fn parse_json_value(input: &str) -> Result<Value, DefinitionError> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let value = NoDuplicateValue
        .deserialize(&mut deserializer)
        .map_err(|err| DefinitionError::Json(err.to_string()))?
        .0;
    deserializer
        .end()
        .map_err(|err| DefinitionError::Json(err.to_string()))?;
    Ok(value)
}

struct NoDuplicateValue;

struct ParsedValue(Value);

impl<'de> DeserializeSeed<'de> for NoDuplicateValue {
    type Value = ParsedValue;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateVisitor)
    }
}

struct NoDuplicateVisitor;

impl<'de> Visitor<'de> for NoDuplicateVisitor {
    type Value = ParsedValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("I-JSON value without duplicate keys or floating-point numbers")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if _value.fract() == 0.0 {
            Err(E::custom("integer number out of range"))
        } else {
            Err(E::custom("floating-point numbers are not allowed"))
        }
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::String(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ParsedValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        NoDuplicateValue.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(NoDuplicateValue)? {
            values.push(value.0);
        }
        Ok(ParsedValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate key `{key}`")));
            }
            let value = map.next_value_seed(NoDuplicateValue)?.0;
            object.insert(key, value);
        }
        Ok(ParsedValue(Value::Object(object)))
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_definition;

    #[test]
    fn canonical_json_fixture_sorts_keys_compacts_integers_and_escapes_strings() {
        let input = r#"{
          "z": 2,
          "a": {"b": "line\nå", "a": 1},
          "arr": [3, {"y": 2, "x": 1}]
        }"#;

        let canonical = canonical_json_string_from_str(input).unwrap();

        assert_eq!(
            canonical,
            "{\"a\":{\"a\":1,\"b\":\"line\\nå\"},\"arr\":[3,{\"x\":1,\"y\":2}],\"z\":2}"
        );
    }

    #[test]
    fn raw_json_canonicalization_rejects_duplicate_keys() {
        let err = canonical_json_string_from_str(r#"{"a":1,"a":2}"#).unwrap_err();
        assert!(err.to_string().contains("duplicate key"));
    }

    #[test]
    fn raw_json_canonicalization_rejects_floating_point_numbers() {
        let err = canonical_json_string_from_str(r#"{"deadband":0.5}"#).unwrap_err();
        assert!(err.to_string().contains("floating-point"));
    }

    #[test]
    fn raw_json_canonicalization_rejects_invalid_numbers() {
        let err = canonical_json_string_from_str(r#"{"value":NaN}"#).unwrap_err();
        assert!(err.to_string().contains("expected value"));
    }

    #[test]
    fn raw_json_canonicalization_rejects_out_of_range_integers() {
        let err = canonical_json_string_from_str(r#"{"value":18446744073709551616}"#).unwrap_err();
        assert!(err.to_string().contains("number out of range"));
    }

    #[test]
    fn content_hash_self_excludes_content_hash_and_truncates_in_digest_order() {
        let mut definition = sample_definition();
        definition.header.content_hash = "not-the-hash".to_string();

        let hash = compute_content_hash(&definition).unwrap();
        let canonical =
            String::from_utf8(canonical_definition_bytes_for_hash(&definition).unwrap()).unwrap();

        assert!(canonical.contains("\"contentHash\":\"\""));
        assert_eq!(hash.content_hash.len(), 64);
        assert!(hash.content_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert!(hash.content_hash.chars().all(|ch| !ch.is_ascii_uppercase()));
        assert_eq!(
            hash.carriage_hash,
            [0x70, 0xdf, 0x73, 0x94, 0xb8, 0x75, 0xe4, 0x92]
        );
        assert_eq!(
            hash.content_hash,
            "70df7394b875e492fd3f7359744939a5c700e7b2c51e744facac31692f3c034a"
        );
    }

    #[test]
    fn definition_hash_preimage_bytes_are_exact_and_self_excluded() {
        let mut definition = sample_definition();
        definition.event_types.clear();
        definition.sources.clear();
        definition.state_machines.clear();
        definition.message_templates.clear();
        definition.enum_sets.clear();
        definition.header.profiles = vec!["Core".to_string()];
        definition.header.conformance_level = "Producer-Core".to_string();
        definition.header.constraints.max_record_size = 128;
        definition.header.constraints.max_slots = 4;
        definition.header.content_hash = "ffffffff".to_string();

        let canonical =
            String::from_utf8(canonical_definition_bytes_for_hash(&definition).unwrap()).unwrap();

        assert_eq!(
            canonical,
            "{\"conditions\":[],\"enumSets\":[],\"eventTypes\":[],\"header\":{\"caps\":{\"crc\":true,\"sourceHighWater\":true},\"conformanceLevel\":\"Producer-Core\",\"constraints\":{\"maxRecordSize\":128,\"maxSlots\":4,\"overflowPolicy\":\"overwrite-oldest\"},\"contentHash\":\"\",\"epochStrategy\":\"retain\",\"profiles\":[\"Core\"],\"semanticVersion\":\"1.0.0\",\"wireVersion\":2},\"messageTemplates\":[],\"severityScale\":{\"high\":{\"max\":1000,\"min\":667},\"low\":{\"max\":332,\"min\":1},\"medium\":{\"max\":666,\"min\":333},\"name\":\"baseline\"},\"sources\":[],\"stateMachines\":[],\"units\":[],\"values\":[{\"dataType\":9,\"deadband\":{\"decimal\":\"0.5\",\"scaled\":null},\"name\":\"Temperature\",\"samplingPolicy\":null,\"semanticRole\":0,\"unit\":null,\"valueId\":2001},{\"dataType\":6,\"deadband\":null,\"name\":\"BatchCount\",\"samplingPolicy\":\"on-change\",\"semanticRole\":3,\"unit\":null,\"valueId\":2002}]}"
        );

        let hash = compute_content_hash(&definition).unwrap();
        assert_eq!(
            hash.content_hash,
            "ecd25cd81846dc108f36c8355aa7296466cc50330bd4ea359909b20f816ee843"
        );
        assert_eq!(
            hash.carriage_hash,
            [0xec, 0xd2, 0x5c, 0xd8, 0x18, 0x46, 0xdc, 0x10]
        );
    }
}
