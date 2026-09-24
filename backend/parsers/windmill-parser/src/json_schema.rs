use serde_json::{json, Map, Value};

use crate::{MainArgSignature, ObjectProperty, Typ};

/// The run-form schema of a script's `main`, derived the way the editor derives it
/// (`inferArgs` in the frontend, `argSigToJsonSchemaType` in `windmill-utils-internal`),
/// so a script deployed without a schema gets the one the editor would have sent.
/// `previous` is the schema of the version being superseded: as in the editor, it is
/// updated in place, so what the user set on it (argument order, descriptions, ...)
/// survives wherever the code still agrees with it.
pub fn main_arg_schema(sig: &MainArgSignature, previous: Option<&Value>) -> Value {
    let mut schema = match previous {
        Some(Value::Object(p)) => p.clone(),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    };
    let mut old_props = match schema.remove("properties") {
        Some(Value::Object(p)) => p,
        _ => Map::new(),
    };
    let mut properties = Map::new();
    let mut required = vec![];
    for arg in &sig.args {
        let mut prop = match old_props.remove(&arg.name) {
            Some(Value::Object(p)) => p,
            _ => fresh_property(),
        };
        if !matches!(arg.typ, Typ::OneOf(_)) {
            prop.remove("oneOf");
        }
        apply_typ(&arg.typ, &mut prop);
        // `T | T[]` marks an argument debouncing accumulates into.
        if let Some(otyp) = arg
            .otyp
            .as_deref()
            .filter(|o| o.contains('[') && o.contains('|'))
        {
            prop.insert("originalType".into(), json!(otyp));
        }
        match &arg.default {
            Some(d) => prop.insert("default".into(), d.clone()),
            None => prop.remove("default"),
        };
        if !arg.has_default {
            required.push(json!(arg.name));
        }
        properties.insert(arg.name.clone(), Value::Object(prop));
    }
    schema.insert("properties".into(), Value::Object(properties));
    schema.insert("required".into(), Value::Array(required));
    if sig.has_cmd_binding == Some(true) {
        schema.insert("x-windmill-ps-cmd-binding".into(), json!(true));
        schema.insert(
            "x-windmill-ps-supports-should-process".into(),
            json!(sig.supports_should_process.unwrap_or(false)),
        );
    } else {
        schema.remove("x-windmill-ps-cmd-binding");
        schema.remove("x-windmill-ps-supports-should-process");
    }
    Value::Object(schema)
}

/// Keys a user sets on a property in the schema editor, kept whatever the code says.
const USER_FIELDS: &[&str] = &[
    "description",
    "pattern",
    "min",
    "max",
    "currency",
    "currencyLocale",
    "multiselect",
    "customErrorMessage",
    "required",
    "showExpr",
    "password",
    "order",
    "dateFormat",
    "title",
    "placeholder",
];

/// Item keys a list the parser cannot see into keeps from the previous schema.
const ITEMS_USER_FIELDS: &[&str] = &[
    "properties",
    "required",
    "additionalProperties",
    "enum",
    "resourceType",
    "contentEncoding",
    "description",
];

fn fresh_property() -> Map<String, Value> {
    let mut p = Map::new();
    p.insert("description".into(), json!(""));
    p.insert("type".into(), json!(""));
    p
}

fn object_properties(
    props: &[ObjectProperty],
    old: Option<&Map<String, Value>>,
) -> Map<String, Value> {
    props
        .iter()
        .map(|p| {
            let mut prop = old
                .and_then(|o| o.get(&p.key))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_else(fresh_property);
            apply_typ(&p.typ, &mut prop);
            (p.key.clone(), Value::Object(prop))
        })
        .collect()
}

fn kept_items(old: &Map<String, Value>) -> Value {
    let old_items = old.get("items").and_then(Value::as_object);
    let mut items = Map::new();
    items.insert(
        "type".into(),
        old_items
            .and_then(|i| i.get("type"))
            .filter(|t| t.as_str().is_some_and(|t| !t.is_empty()))
            .cloned()
            .unwrap_or(json!("object")),
    );
    if let Some(old_items) = old_items {
        for f in ITEMS_USER_FIELDS {
            if let Some(v) = old_items.get(*f) {
                items.insert(f.to_string(), v.clone());
            }
        }
    }
    Value::Object(items)
}

/// Port of `argSigToJsonSchemaType`: rewrites `old` in place to carry `typ`.
fn apply_typ(typ: &Typ, old: &mut Map<String, Value>) {
    let mut new = Map::new();
    // Keys the editor sets to `undefined`, which drops them from the stored JSON.
    let mut unset: Vec<&str> = vec![];
    let mut set = |k: &str, v: Value| {
        new.insert(k.to_string(), v);
    };
    match typ {
        Typ::Int => set("type", json!("integer")),
        Typ::Float => set("type", json!("number")),
        Typ::Bool => set("type", json!("boolean")),
        Typ::Email => {
            set("type", json!("string"));
            set("format", json!("email"));
        }
        Typ::Sql => {
            set("type", json!("string"));
            set("format", json!("sql"));
        }
        Typ::Bytes => {
            set("type", json!("string"));
            set("contentEncoding", json!("base64"));
            set("originalType", json!("bytes"));
        }
        Typ::Datetime => {
            set("type", json!("string"));
            set("format", json!("date-time"));
        }
        Typ::Date => {
            set("type", json!("string"));
            set("format", json!("date"));
        }
        Typ::OneOf(variants) => {
            set("type", json!("object"));
            let old_variants = old.get("oneOf").and_then(Value::as_array);
            let one_of: Vec<Value> = variants
                .iter()
                .map(|v| {
                    let old_variant = old_variants
                        .and_then(|o| o.iter().find(|o| o.get("title") == Some(&json!(v.label))))
                        .and_then(Value::as_object);
                    let old_props = old_variant
                        .and_then(|o| o.get("properties"))
                        .and_then(Value::as_object);
                    let mut variant = json!({
                        "type": "object",
                        "title": v.label,
                        "properties": object_properties(&v.properties, old_props),
                    });
                    if let Some(order) = old_variant.and_then(|o| o.get("order")) {
                        variant["order"] = order.clone();
                    }
                    variant
                })
                .collect();
            set("oneOf", Value::Array(one_of));
        }
        Typ::Object(o) => {
            set("type", json!("object"));
            if let Some(name) = &o.name {
                set("format", json!(format!("resource-{name}")));
            }
            if let Some(props) = &o.props {
                let old_props = old.get("properties").and_then(Value::as_object);
                set(
                    "properties",
                    Value::Object(object_properties(props, old_props)),
                );
            }
        }
        Typ::Str(variants) => {
            set("type", json!("string"));
            match variants {
                Some(v) => {
                    set("originalType", json!("enum"));
                    set("enum", json!(v));
                }
                None => {
                    set("originalType", json!("string"));
                    match old.get("enum") {
                        Some(e) if old.get("originalType") == Some(&json!("string")) => {
                            set("enum", e.clone())
                        }
                        _ => unset.push("enum"),
                    }
                }
            }
        }
        Typ::Resource(name) => {
            set("type", json!("object"));
            set("format", json!(format!("resource-{name}")));
        }
        Typ::DynSelect(name) => {
            set("type", json!("object"));
            set("format", json!(format!("dynselect-{name}")));
        }
        Typ::DynMultiselect(name) => {
            set("type", json!("object"));
            set("format", json!(format!("dynmultiselect-{name}")));
        }
        Typ::List(inner) => {
            set("type", json!("array"));
            let (items, original_type) = match inner.as_ref() {
                Typ::Int | Typ::Float => (json!({"type": "number"}), "number[]"),
                Typ::Bytes => (
                    json!({"type": "string", "contentEncoding": "base64"}),
                    "bytes[]",
                ),
                Typ::Str(Some(v)) => (json!({"type": "string", "enum": v}), "enum[]"),
                Typ::Str(None) => {
                    let mut items = json!({"type": "string"});
                    if let Some(e) = old.get("items").and_then(|i| i.get("enum")) {
                        items["enum"] = e.clone();
                    }
                    (items, "string[]")
                }
                Typ::Resource(r) => (json!({"type": "resource", "resourceType": r}), "resource[]"),
                Typ::Object(o) => {
                    if let Some(name) = &o.name {
                        set("format", json!(format!("resource-{name}")));
                    }
                    let items = match o.props.as_deref().filter(|p| !p.is_empty()) {
                        Some(props) => json!({
                            "type": "object",
                            "properties": object_properties(props, None),
                        }),
                        None => kept_items(old),
                    };
                    (items, "record[]")
                }
                _ => (kept_items(old), "object[]"),
            };
            set("items", items);
            set("originalType", json!(original_type));
        }
        // The parser cannot tell: whatever type the user gave the argument stands.
        Typ::Unknown => set("type", old.get("type").cloned().unwrap_or(json!("object"))),
    }

    for f in USER_FIELDS {
        if let Some(v) = old.get(*f) {
            new.insert(f.to_string(), v.clone());
        }
    }

    let old_type = old.get("type").cloned();
    let new_type = new.get("type").cloned();
    let old_format = old
        .get("format")
        .and_then(Value::as_str)
        .map(str::to_string);
    let new_format = new
        .get("format")
        .and_then(Value::as_str)
        .map(str::to_string);
    let is_date = |f: &Option<String>| matches!(f.as_deref(), Some("date" | "date-time"));
    let mut keep_old_format = false;
    if old_type != new_type {
    } else if new_format.as_deref() == Some("date-time") && old_format.as_deref() == Some("date") {
        new.insert("format".into(), json!("date"));
    } else if is_date(&new_format) {
        keep_old_format = true;
    } else if old.get("items").and_then(|i| i.get("type"))
        != new.get("items").and_then(|i| i.get("type"))
    {
        old.remove("items");
    } else if old_type == Some(json!("string")) && old_format.is_some() {
        keep_old_format = true;
    }
    if keep_old_format {
        match old.get("format") {
            Some(f) => {
                new.insert("format".into(), f.clone());
            }
            None => {
                new.remove("format");
                unset.push("format");
            }
        }
    }
    if (old_type != new_type || new_type != Some(json!("string"))) && !new.contains_key("format") {
        old.remove("format");
    }

    for k in unset {
        if !new.contains_key(k) {
            old.remove(k);
        }
    }
    old.extend(new);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arg, ObjectType};

    fn arg(name: &str, typ: Typ, default: Option<Value>) -> Arg {
        Arg {
            name: name.to_string(),
            has_default: default.is_some(),
            default,
            typ,
            ..Default::default()
        }
    }

    #[test]
    fn keeps_what_the_user_set_on_the_previous_version() {
        let sig = MainArgSignature {
            args: vec![
                arg("n", Typ::Int, None),
                arg("db", Typ::Resource("postgresql".into()), None),
                arg(
                    "tags",
                    Typ::List(Box::new(Typ::Str(None))),
                    Some(json!(["a"])),
                ),
                arg(
                    "opts",
                    Typ::Object(ObjectType::new(None, None)),
                    Some(json!({})),
                ),
                arg("any", Typ::Unknown, None),
            ],
            ..Default::default()
        };
        let previous = json!({
            "type": "object",
            "order": ["db", "n"],
            "properties": {
                "n": {"type": "integer", "description": "how many", "min": 1, "nullable": true},
                "tags": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}},
                "gone": {"type": "string"},
            },
        });
        let schema = main_arg_schema(&sig, Some(&previous));
        assert_eq!(schema["order"], json!(["db", "n"]));
        assert_eq!(schema["required"], json!(["n", "db", "any"]));
        assert_eq!(
            schema["properties"]["n"],
            json!({"type": "integer", "description": "how many", "min": 1, "nullable": true})
        );
        assert!(schema["properties"].get("gone").is_none());
        assert_eq!(schema["properties"]["db"]["format"], "resource-postgresql");
        assert_eq!(
            schema["properties"]["tags"]["items"],
            json!({"type": "string", "enum": ["a", "b"]})
        );
        assert_eq!(schema["properties"]["tags"]["default"], json!(["a"]));
        assert_eq!(schema["properties"]["opts"]["type"], "object");
        // Untyped in the code and new: any JSON value, as the editor stores it.
        assert_eq!(schema["properties"]["any"]["type"], "");
    }
}
