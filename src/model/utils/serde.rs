//! Serde utilities for serializing and deserializing data.

use serde::{Deserialize, Deserializer, Serialize};

/// Accept extractor timestamps expressed as integer or fractional Unix seconds.
/// Subsecond precision is discarded because metadata dates use whole seconds.
pub fn unix_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Timestamp {
        Integer(i64),
        Decimal(f64),
    }

    match Option::<Timestamp>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Timestamp::Integer(value)) => Ok(Some(value)),
        Some(Timestamp::Decimal(value)) => {
            // i64::MAX rounds up to 2^63 as f64, so the upper bound is exclusive.
            if !value.is_finite() || value < i64::MIN as f64 || value >= -(i64::MIN as f64) {
                return Err(serde::de::Error::custom("Unix timestamp is outside the i64 range"));
            }
            Ok(Some(value.trunc() as i64))
        }
    }
}

/// Fix issue with 'none' string in JSON.
pub fn json_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let string: Option<String> = Option::deserialize(deserializer)?;

    match string.as_deref() {
        Some("none") => Ok(None),
        _ => Ok(string),
    }
}

/// Serializes a value to a JSON string, returning an empty string on failure.
///
/// # Arguments
///
/// * `value` - The value to serialize
///
/// # Returns
///
/// The JSON string representation, or an empty string if serialization fails.
pub fn serialize_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Serializes an optional value to an optional JSON string, returning `None` if the input is `None`.
///
/// # Arguments
///
/// * `value` - The optional value to serialize
///
/// # Returns
///
/// `Some(json_string)` if the value is `Some`, `None` otherwise.
/// On serialization failure, returns `Some("")`.
pub fn serialize_json_opt<T: Serialize>(value: Option<T>) -> Option<String> {
    value.map(|v| serde_json::to_string(&v).unwrap_or_default())
}
