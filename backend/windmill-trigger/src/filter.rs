use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::fmt;

#[derive(Deserialize)]
pub struct JsonFilter {
    pub key: String,
    pub value: Value,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Filter {
    JsonFilter(JsonFilter),
}

struct SupersetVisitor<'a> {
    key: &'a str,
    value_to_check: &'a Value,
}

impl<'de, 'a> Visitor<'de> for SupersetVisitor<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON object with a specific key at the top level")
    }

    fn visit_map<V>(self, mut map: V) -> std::result::Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut result = false;
        let mut found = false;

        // Must consume entire map to satisfy deserializer contract
        while let Some(key) = map.next_key::<String>()? {
            if !found && key == self.key {
                let json_value: Value = map.next_value()?;
                result = is_superset(&json_value, self.value_to_check);
                found = true;
            } else {
                // Skip values we don't need (cheaper than full deserialization)
                let _ = map.next_value::<de::IgnoredAny>()?;
            }
        }
        Ok(result)
    }
}

pub fn is_superset(json_value: &Value, value_to_check: &Value) -> bool {
    match (json_value, value_to_check) {
        (Value::Object(json_map), Value::Object(check_map)) => check_map.iter().all(|(k, v)| {
            json_map
                .get(k)
                .map_or(false, |json_val| is_superset(json_val, v))
        }),
        (Value::Array(json_array), Value::Array(check_array)) => {
            check_array.iter().all(|check_item| {
                json_array
                    .iter()
                    .any(|json_item| is_superset(json_item, check_item))
            })
        }
        _ => json_value == value_to_check,
    }
}

pub fn is_value_superset<'a, 'de, D>(
    deserializer: D,
    key: &'a str,
    value_to_check: &'a Value,
) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_map(SupersetVisitor { key, value_to_check })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_with_other_top_level_keys() {
        let payload = r#"{"event_type": "test", "other": "data"}"#;
        let key = "event_type";
        let value = json!("test");

        let mut deserializer = serde_json::Deserializer::from_str(payload);
        let result = is_value_superset(&mut deserializer, key, &value).unwrap();
        assert!(result, "Should match when key exists with correct value");
    }

    #[test]
    fn test_filter_with_key_not_first() {
        let payload = r#"{"other": "data", "event_type": "test"}"#;
        let key = "event_type";
        let value = json!("test");

        let mut deserializer = serde_json::Deserializer::from_str(payload);
        let result = is_value_superset(&mut deserializer, key, &value).unwrap();
        assert!(result, "Should match even when key is not first");
    }

    #[test]
    fn test_filter_with_nested_object() {
        let payload = r#"{"data": {"status": "active", "count": 5}, "other": "value"}"#;
        let key = "data";
        let value = json!({"status": "active"});

        let mut deserializer = serde_json::Deserializer::from_str(payload);
        let result = is_value_superset(&mut deserializer, key, &value).unwrap();
        assert!(result, "Should match when nested object is superset");
    }

    #[test]
    fn test_filter_no_match() {
        let payload = r#"{"event_type": "other", "data": "value"}"#;
        let key = "event_type";
        let value = json!("test");

        let mut deserializer = serde_json::Deserializer::from_str(payload);
        let result = is_value_superset(&mut deserializer, key, &value).unwrap();
        assert!(!result, "Should not match when value differs");
    }

    #[test]
    fn test_filter_key_not_found() {
        let payload = r#"{"other": "data"}"#;
        let key = "event_type";
        let value = json!("test");

        let mut deserializer = serde_json::Deserializer::from_str(payload);
        let result = is_value_superset(&mut deserializer, key, &value).unwrap();
        assert!(!result, "Should not match when key doesn't exist");
    }

    // --- is_superset unit tests ---

    #[test]
    fn test_superset_equal_scalars() {
        assert!(is_superset(&json!(42), &json!(42)));
        assert!(is_superset(&json!("hello"), &json!("hello")));
        assert!(is_superset(&json!(true), &json!(true)));
        assert!(is_superset(&json!(null), &json!(null)));
    }

    #[test]
    fn test_superset_unequal_scalars() {
        assert!(!is_superset(&json!(42), &json!(43)));
        assert!(!is_superset(&json!("hello"), &json!("world")));
        assert!(!is_superset(&json!(true), &json!(false)));
    }

    #[test]
    fn test_superset_object_subset() {
        let full = json!({"a": 1, "b": 2, "c": 3});
        let subset = json!({"a": 1, "b": 2});
        assert!(is_superset(&full, &subset));
    }

    #[test]
    fn test_superset_object_not_subset() {
        let full = json!({"a": 1, "b": 2});
        let check = json!({"a": 1, "b": 3});
        assert!(!is_superset(&full, &check));
    }

    #[test]
    fn test_superset_object_missing_key() {
        let full = json!({"a": 1});
        let check = json!({"a": 1, "b": 2});
        assert!(!is_superset(&full, &check));
    }

    #[test]
    fn test_superset_nested_objects() {
        let full = json!({"a": {"b": {"c": 1, "d": 2}, "e": 3}});
        let check = json!({"a": {"b": {"c": 1}}});
        assert!(is_superset(&full, &check));
    }

    #[test]
    fn test_superset_array_subset() {
        let full = json!([1, 2, 3, 4]);
        let check = json!([2, 4]);
        assert!(is_superset(&full, &check));
    }

    #[test]
    fn test_superset_array_not_subset() {
        let full = json!([1, 2, 3]);
        let check = json!([4]);
        assert!(!is_superset(&full, &check));
    }

    #[test]
    fn test_superset_empty_check() {
        assert!(is_superset(&json!({"a": 1}), &json!({})));
        assert!(is_superset(&json!([1, 2]), &json!([])));
    }

    #[test]
    fn test_superset_array_of_objects() {
        let full = json!([{"id": 1, "name": "a"}, {"id": 2, "name": "b"}]);
        let check = json!([{"id": 1}]);
        assert!(is_superset(&full, &check));
    }

    #[test]
    fn test_superset_type_mismatch() {
        assert!(!is_superset(&json!(42), &json!("42")));
        assert!(!is_superset(&json!([1]), &json!(1)));
    }
}
