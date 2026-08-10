use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;
use std::{collections::HashMap, fmt};

#[derive(Debug, Deserialize)]
pub struct JsonFilter {
    pub key: String,
    pub value: Value,
}

/// Boolean group of nested filters, externally tagged (`{"any_of": [...]}`) so it is
/// unambiguous against a leaf filter, which is `{"key": ..., "value": ...}`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterGroup {
    AnyOf(Vec<Filter>),
    AllOf(Vec<Filter>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    JsonFilter(JsonFilter),
    Group(FilterGroup),
}

/// Filters prepared for repeated evaluation against a stream of messages. The set of
/// top-level keys the whole tree references is computed once, so each message is parsed
/// in a single pass instead of once per leaf filter.
#[derive(Debug, Default)]
pub struct CompiledFilters {
    filters: Vec<Filter>,
    use_or_logic: bool,
    keys: Vec<String>,
}

impl CompiledFilters {
    pub fn new(filters: Vec<Filter>, use_or_logic: bool) -> Self {
        let mut keys = Vec::new();
        collect_keys(&filters, &mut keys);
        Self { filters, use_or_logic, keys }
    }

    /// Build from the raw JSON of each filter as stored in the trigger config. An entry
    /// that fails to parse is skipped rather than dropping the other filters, but it
    /// widens what the trigger accepts, so it is reported.
    pub fn parse<'a>(
        raw_filters: impl IntoIterator<Item = &'a str>,
        use_or_logic: bool,
        trigger_path: &str,
    ) -> Self {
        let filters = raw_filters
            .into_iter()
            .filter_map(|raw| match serde_json::from_str::<Filter>(raw) {
                Ok(filter) => Some(filter),
                Err(err) => {
                    tracing::error!(
                        "Ignoring unparseable filter of trigger {}: {} ({})",
                        trigger_path,
                        raw,
                        err
                    );
                    None
                }
            })
            .collect();
        Self::new(filters, use_or_logic)
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Whether `text`, parsed as a JSON object, satisfies the filters.
    pub fn matches(&self, text: &str) -> bool {
        if self.filters.is_empty() {
            return true;
        }

        let mut deserializer = serde_json::Deserializer::from_str(text);
        let values =
            Deserializer::deserialize_map(&mut deserializer, KeysVisitor { keys: &self.keys })
                .unwrap_or_default();

        eval_all(&self.filters, self.use_or_logic, &values)
    }
}

fn collect_keys(filters: &[Filter], keys: &mut Vec<String>) {
    for filter in filters {
        match filter {
            Filter::JsonFilter(JsonFilter { key, .. }) => {
                if !keys.iter().any(|k| k == key) {
                    keys.push(key.clone());
                }
            }
            Filter::Group(FilterGroup::AnyOf(nested) | FilterGroup::AllOf(nested)) => {
                collect_keys(nested, keys)
            }
        }
    }
}

fn eval_all(filters: &[Filter], use_or_logic: bool, values: &HashMap<&str, Value>) -> bool {
    // An empty group adds no constraint, as an empty top-level filter list does: an
    // `any_of: []` evaluating to false would silently mute the trigger.
    if filters.is_empty() {
        return true;
    }

    let eval = |filter: &Filter| match filter {
        Filter::JsonFilter(JsonFilter { key, value }) => values
            .get(key.as_str())
            .map_or(false, |found| is_superset(found, value)),
        Filter::Group(FilterGroup::AnyOf(nested)) => eval_all(nested, true, values),
        Filter::Group(FilterGroup::AllOf(nested)) => eval_all(nested, false, values),
    };

    if use_or_logic {
        filters.iter().any(eval)
    } else {
        filters.iter().all(eval)
    }
}

/// Collects the values of the requested top-level keys in a single pass, skipping every
/// other value instead of materializing the whole message.
struct KeysVisitor<'k> {
    keys: &'k [String],
}

impl<'de, 'k> Visitor<'de> for KeysVisitor<'k> {
    type Value = HashMap<&'k str, Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<V>(self, mut map: V) -> std::result::Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut found = HashMap::with_capacity(self.keys.len());

        // Must consume entire map to satisfy deserializer contract
        while let Some(key) = map.next_key::<String>()? {
            match self.keys.iter().find(|k| k.as_str() == key) {
                // On a duplicated key the first occurrence wins
                Some(k) if !found.contains_key(k.as_str()) => {
                    found.insert(k.as_str(), map.next_value::<Value>()?);
                }
                _ => {
                    // Skip values we don't need (cheaper than full deserialization)
                    let _ = map.next_value::<de::IgnoredAny>()?;
                }
            }
        }

        Ok(found)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn matches(payload: &str, filters: serde_json::Value, use_or_logic: bool) -> bool {
        let filters: Vec<Filter> = serde_json::from_value(filters).unwrap();
        CompiledFilters::new(filters, use_or_logic).matches(payload)
    }

    #[test]
    fn test_filter_with_other_top_level_keys() {
        let payload = r#"{"event_type": "test", "other": "data"}"#;
        let filters = json!([{"key": "event_type", "value": "test"}]);
        assert!(
            matches(payload, filters, false),
            "Should match when key exists with correct value"
        );
    }

    #[test]
    fn test_filter_with_key_not_first() {
        let payload = r#"{"other": "data", "event_type": "test"}"#;
        let filters = json!([{"key": "event_type", "value": "test"}]);
        assert!(
            matches(payload, filters, false),
            "Should match even when key is not first"
        );
    }

    #[test]
    fn test_filter_with_nested_object() {
        let payload = r#"{"data": {"status": "active", "count": 5}, "other": "value"}"#;
        let filters = json!([{"key": "data", "value": {"status": "active"}}]);
        assert!(
            matches(payload, filters, false),
            "Should match when nested object is superset"
        );
    }

    #[test]
    fn test_filter_no_match() {
        let payload = r#"{"event_type": "other", "data": "value"}"#;
        let filters = json!([{"key": "event_type", "value": "test"}]);
        assert!(
            !matches(payload, filters, false),
            "Should not match when value differs"
        );
    }

    #[test]
    fn test_filter_key_not_found() {
        let payload = r#"{"other": "data"}"#;
        let filters = json!([{"key": "event_type", "value": "test"}]);
        assert!(
            !matches(payload, filters, false),
            "Should not match when key doesn't exist"
        );
    }

    #[test]
    fn test_no_filters_matches_everything() {
        assert!(matches(r#"{"a": 1}"#, json!([]), false));
        assert!(matches("not even json", json!([]), true));
    }

    #[test]
    fn test_non_object_payload_never_matches() {
        let filters = json!([{"key": "a", "value": 1}]);
        assert!(!matches("[1, 2]", filters.clone(), false));
        assert!(!matches("nope", filters, true));
    }

    #[test]
    fn test_top_level_and_or_logic() {
        let payload = r#"{"a": 1, "b": 2}"#;
        let filters = json!([{"key": "a", "value": 1}, {"key": "b", "value": 99}]);
        assert!(!matches(payload, filters.clone(), false));
        assert!(matches(payload, filters, true));
    }

    // --- nested groups ---

    #[test]
    fn test_any_of_group_nested_in_and() {
        let payload =
            r#"{"event": "message_created", "previous_message": {"sent_by": "reminder"}}"#;
        let filters = json!([
            {"key": "event", "value": "message_created"},
            {"any_of": [
                {"key": "in_reply_to", "value": {"sent_by": "reminder"}},
                {"key": "previous_message", "value": {"sent_by": "reminder"}}
            ]}
        ]);
        assert!(matches(payload, filters, false));
    }

    #[test]
    fn test_any_of_group_all_branches_fail() {
        let payload = r#"{"event": "message_created", "previous_message": {"sent_by": "someone"}}"#;
        let filters = json!([
            {"key": "event", "value": "message_created"},
            {"any_of": [
                {"key": "in_reply_to", "value": {"sent_by": "reminder"}},
                {"key": "previous_message", "value": {"sent_by": "reminder"}}
            ]}
        ]);
        assert!(!matches(payload, filters, false));
    }

    #[test]
    fn test_all_of_group_nested_in_or() {
        let payload = r#"{"a": 1, "b": 2}"#;
        let filters = json!([
            {"key": "missing", "value": true},
            {"all_of": [{"key": "a", "value": 1}, {"key": "b", "value": 2}]}
        ]);
        assert!(matches(payload, filters.clone(), true));
        assert!(!matches(payload, filters, false));
    }

    #[test]
    fn test_deeply_nested_groups() {
        let payload = r#"{"a": 1, "b": 2, "c": 3}"#;
        let filters = json!([
            {"any_of": [
                {"key": "a", "value": 99},
                {"all_of": [
                    {"key": "b", "value": 2},
                    {"any_of": [{"key": "c", "value": 3}, {"key": "c", "value": 4}]}
                ]}
            ]}
        ]);
        assert!(matches(payload, filters, false));
    }

    #[test]
    fn test_empty_group_adds_no_constraint() {
        let payload = r#"{"a": 1}"#;
        assert!(matches(payload, json!([{"any_of": []}]), false));
        assert!(matches(payload, json!([{"all_of": []}]), false));
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
