use serde_json::{json, Map, Value};

use crate::{Arg, MainArgSignature, Typ};

/// Keys a user sets on a property in the schema editor. They survive a
/// re-inference, as they do when the editor re-parses the code.
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

/// The run-form schema of a script's `main`, built the way the editor builds it
/// (`argSigToJsonSchemaType` in `windmill-utils-internal`), so a script deployed
/// without a schema gets the one the editor would have sent. `previous` is the
/// schema of the version being superseded, whose per-argument annotations are kept.
pub fn main_arg_schema(sig: &MainArgSignature, previous: Option<&Value>) -> Value {
    let previous_props = previous
        .and_then(|p| p.get("properties"))
        .and_then(Value::as_object);
    let mut properties = Map::new();
    let mut required = vec![];
    for arg in &sig.args {
        let mut prop = arg_property(arg);
        if let (Some(old), Some(new)) = (
            previous_props
                .and_then(|p| p.get(&arg.name))
                .and_then(Value::as_object),
            prop.as_object_mut(),
        ) {
            keep_user_fields(old, new);
        }
        properties.insert(arg.name.clone(), prop);
        if !arg.has_default {
            required.push(json!(arg.name));
        }
    }
    let mut schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": properties,
        "required": required,
    });
    if sig.has_cmd_binding == Some(true) {
        schema["x-windmill-ps-cmd-binding"] = json!(true);
        schema["x-windmill-ps-supports-should-process"] =
            json!(sig.supports_should_process.unwrap_or(false));
    }
    schema
}

fn keep_user_fields(old: &Map<String, Value>, new: &mut Map<String, Value>) {
    for field in USER_FIELDS {
        if let Some(v) = old.get(*field) {
            new.insert(field.to_string(), v.clone());
        }
    }
    // An argument the code leaves untyped keeps whatever type the user gave it.
    if new.get("type") == Some(&json!("")) {
        if let Some(t) = old.get("type") {
            new.insert("type".to_string(), t.clone());
        }
    }
    // A format or enum the user picked for a plain string (e.g. `password`,
    // a fixed list of choices) is theirs as long as the code still says string.
    if old.get("type") == new.get("type") && new.get("type") == Some(&json!("string")) {
        for field in ["format", "enum"] {
            if !new.contains_key(field) {
                if let Some(v) = old.get(field) {
                    new.insert(field.to_string(), v.clone());
                }
            }
        }
    }
}

fn arg_property(arg: &Arg) -> Value {
    let mut prop = typ_property(&arg.typ);
    // `T | T[]` marks an argument debouncing accumulates into.
    if let Some(otyp) = arg
        .otyp
        .as_deref()
        .filter(|o| o.contains('[') && o.contains('|'))
    {
        prop["originalType"] = json!(otyp);
    }
    if let Some(default) = &arg.default {
        prop["default"] = default.clone();
    }
    prop
}

fn object_properties(props: &[crate::ObjectProperty]) -> Map<String, Value> {
    props
        .iter()
        .map(|p| (p.key.clone(), typ_property(&p.typ)))
        .collect()
}

fn typ_property(typ: &Typ) -> Value {
    let mut prop = match typ {
        Typ::Int => json!({"type": "integer"}),
        Typ::Float => json!({"type": "number"}),
        Typ::Bool => json!({"type": "boolean"}),
        Typ::Email => json!({"type": "string", "format": "email"}),
        Typ::Sql => json!({"type": "string", "format": "sql"}),
        Typ::Bytes => {
            json!({"type": "string", "contentEncoding": "base64", "originalType": "bytes"})
        }
        Typ::Datetime => json!({"type": "string", "format": "date-time"}),
        Typ::Date => json!({"type": "string", "format": "date"}),
        Typ::Str(Some(variants)) => {
            json!({"type": "string", "originalType": "enum", "enum": variants})
        }
        Typ::Str(None) => json!({"type": "string", "originalType": "string"}),
        Typ::Resource(name) => json!({"type": "object", "format": format!("resource-{name}")}),
        Typ::DynSelect(name) => json!({"type": "object", "format": format!("dynselect-{name}")}),
        Typ::DynMultiselect(name) => {
            json!({"type": "object", "format": format!("dynmultiselect-{name}")})
        }
        Typ::Object(o) => {
            let mut p = json!({"type": "object"});
            if let Some(name) = &o.name {
                p["format"] = json!(format!("resource-{name}"));
            }
            if let Some(props) = &o.props {
                p["properties"] = Value::Object(object_properties(props));
            }
            p
        }
        Typ::OneOf(variants) => {
            let one_of: Vec<Value> = variants
                .iter()
                .map(|v| {
                    json!({
                        "type": "object",
                        "title": v.label,
                        "properties": object_properties(&v.properties),
                    })
                })
                .collect();
            json!({"type": "object", "oneOf": one_of})
        }
        Typ::List(inner) => list_property(inner),
        // Any JSON value: the empty type is the editor's, and not "object", which
        // would refuse a string or a number for an untyped argument.
        Typ::Unknown => json!({"type": ""}),
    };
    prop["description"] = json!("");
    prop
}

fn list_property(inner: &Typ) -> Value {
    let (items, original_type, format) = match inner {
        Typ::Int | Typ::Float => (json!({"type": "number"}), "number[]", None),
        Typ::Bytes => (
            json!({"type": "string", "contentEncoding": "base64"}),
            "bytes[]",
            None,
        ),
        Typ::Str(Some(variants)) => (json!({"type": "string", "enum": variants}), "enum[]", None),
        Typ::Str(None) => (json!({"type": "string"}), "string[]", None),
        Typ::Resource(name) => (
            json!({"type": "resource", "resourceType": name}),
            "resource[]",
            None,
        ),
        Typ::Object(o) => {
            let mut items = json!({"type": "object"});
            if let Some(props) = o.props.as_deref().filter(|p| !p.is_empty()) {
                items["properties"] = Value::Object(object_properties(props));
            }
            (
                items,
                "record[]",
                o.name.as_ref().map(|n| format!("resource-{n}")),
            )
        }
        _ => (json!({"type": "object"}), "object[]", None),
    };
    let mut prop = json!({"type": "array", "items": items, "originalType": original_type});
    if let Some(format) = format {
        prop["format"] = json!(format);
    }
    prop
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObjectType;

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
    fn keeps_annotations_of_the_previous_version() {
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
            ],
            ..Default::default()
        };
        let previous = json!({"properties": {
            "n": {"type": "integer", "description": "how many", "min": 1},
        }});
        let schema = main_arg_schema(&sig, Some(&previous));
        assert_eq!(schema["required"], json!(["n", "db"]));
        assert_eq!(
            schema["properties"]["n"],
            json!({"type": "integer", "description": "how many", "min": 1})
        );
        assert_eq!(schema["properties"]["db"]["format"], "resource-postgresql");
        assert_eq!(
            schema["properties"]["tags"]["items"],
            json!({"type": "string"})
        );
        assert_eq!(schema["properties"]["tags"]["default"], json!(["a"]));
        assert_eq!(schema["properties"]["opts"]["type"], "object");
    }
}
