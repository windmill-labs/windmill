use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use serde_json::{value::RawValue, Value};

use crate::{error::Error, scripts::ScriptLang};

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub enum JsonPrimitiveType {
    String,
    Number,
    Integer,
    Object,
    Array,
    Boolean,
    Null,
}

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub enum SchemaValidationRule {
    StrictEnum(Vec<Value>),
    IsNull,
    IsInteger,
    IsString,
    IsBool,
    IsDatetime,
    IsNumber,
    IsEmail,
    IsObject(Vec<(String, Vec<SchemaValidationRule>)>),
    IsArray(Vec<SchemaValidationRule>),
    IsUnionType(Vec<Vec<SchemaValidationRule>>),
    IsBytes,
}

impl SchemaValidationRule {
    fn from_primitive(p: &JsonPrimitiveType, val: &Value) -> Result<Vec<Self>, anyhow::Error> {
        let mut schema_rules = vec![];

        match p {
            JsonPrimitiveType::String => {
                schema_rules.push(SchemaValidationRule::IsString);

                if let Some(format) = val.get("format").and_then(|f| f.as_str()) {
                    if format == "date" || format == "date-time" {
                        schema_rules.push(SchemaValidationRule::IsDatetime);
                    }

                    if format == "email" {
                        schema_rules.push(SchemaValidationRule::IsEmail);
                    }
                }

                if let Some(encoding) = val.get("contentEncoding").and_then(|e| e.as_str()) {
                    if encoding == "base64" {
                        schema_rules.push(SchemaValidationRule::IsBytes);
                    }
                }
            }

            JsonPrimitiveType::Number => {
                schema_rules.push(SchemaValidationRule::IsNumber);
            }
            JsonPrimitiveType::Integer => {
                schema_rules.push(SchemaValidationRule::IsInteger);
            }
            JsonPrimitiveType::Object => {
                let mut obj_rules = vec![];

                if let Some(properties) = val.get("properties") {
                    let properties = properties
                        .as_object()
                        .ok_or(anyhow!("Field properties should be an object"))?;

                    for (key, v) in properties {
                        obj_rules.push((key.clone(), SchemaValidationRule::from_value(v)?))
                    }

                    schema_rules.push(SchemaValidationRule::IsObject(obj_rules));
                } else if let Some(one_of) = val.get("oneOf") {
                    let one_of = one_of
                        .as_array()
                        .ok_or(anyhow!("`oneOf` needs to be an array"))?;
                    let mut rules = vec![];

                    for variant in one_of {
                        rules.push(SchemaValidationRule::from_value(variant)?);
                    }

                    schema_rules.push(SchemaValidationRule::IsUnionType(rules))
                } else {
                    let is_resource = val
                        .get("format")
                        .and_then(|f| f.as_str())
                        .map(|f| f.starts_with("resource"))
                        .unwrap_or(false);
                    if !is_resource {
                        return Err(anyhow!(
                        "Object type should have a `properties` or `anyOf` field, or be a resource"
                        ));
                    }
                }
            }
            JsonPrimitiveType::Array => {
                let items = val
                    .get("items")
                    .ok_or(anyhow!("Array type should have field `items`"))?;

                let arr_rules = SchemaValidationRule::from_value(items)?;

                schema_rules.push(SchemaValidationRule::IsArray(arr_rules));
            }
            JsonPrimitiveType::Boolean => {
                schema_rules.push(SchemaValidationRule::IsBool);
            }
            JsonPrimitiveType::Null => {
                schema_rules.push(SchemaValidationRule::IsNull);
            }
        }

        Ok(schema_rules)
    }

    fn from_value(val: &Value) -> Result<Vec<Self>, Error> {
        if let Some(any_of) = val.get("anyOf").and_then(|any_of| any_of.as_array()) {
            let mut r = vec![];

            for variant in any_of {
                r.push(SchemaValidationRule::from_value(variant)?);
            }
            return Ok(vec![SchemaValidationRule::IsUnionType(r)]);
        }

        let mut schema_rules = vec![];

        let typ = val.get("type").ok_or(anyhow!("Missing `type` field"))?;

        if let Some(typ) = typ.as_str() {
            schema_rules.append(&mut SchemaValidationRule::from_primitive(
                &JsonPrimitiveType::from_str(typ)?,
                val,
            )?);
        } else if let Some(typ_arr) = typ.as_array() {
            let typ_arr = typ_arr
                .into_iter()
                .map(|v| {
                    SchemaValidationRule::from_primitive(
                        &JsonPrimitiveType::from_str(
                            v.as_str()
                                .ok_or(anyhow!("Expected array of strings for `type` field"))?,
                        )?,
                        v,
                    )
                })
                .collect::<Result<Vec<Vec<SchemaValidationRule>>, anyhow::Error>>()?;

            schema_rules.push(SchemaValidationRule::IsUnionType(typ_arr));
        } else {
            return Err(anyhow!(
                "Unsupported value for type field, expected string or string array"
            )
            .into());
        }

        if let Some(enum_variants) = val.get("enum") {
            let variants = enum_variants
                .as_array()
                .ok_or(anyhow!("enum variants are not in an array"))?
                .clone();
            schema_rules.push(SchemaValidationRule::StrictEnum(variants));
        }

        Ok(schema_rules)
    }

    fn apply_rule(&self, key: &str, val: &Value) -> Result<(), Error> {
        match self {
            SchemaValidationRule::IsNull => {
                if !val.is_null() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be null"
                    )));
                }
            }
            SchemaValidationRule::StrictEnum(vec) => {
                if !vec.contains(val) {
                    return Err(Error::ArgumentErr(format!(
                        "Enum type argument `{key}` expected one of `{vec:?}` but received {val:?}"
                    )));
                }
            }
            SchemaValidationRule::IsNumber => {
                if !val.is_number() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be a numeric value"
                    )));
                }
            }
            SchemaValidationRule::IsInteger => {
                if !val.is_i64() && !val.is_u64() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be an integer"
                    )));
                }
            }
            SchemaValidationRule::IsString => {
                if !val.is_string() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be a string"
                    )));
                }
            }
            SchemaValidationRule::IsBool => {
                if !val.is_boolean() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be a boolean"
                    )));
                }
            }
            SchemaValidationRule::IsObject(o) => {
                if !val.is_object() {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be an object"
                    )));
                }

                for (s, rules) in o {
                    if let Some(v) = val.get(&s) {
                        for r in rules {
                            r.apply_rule(&format!("{key}.{s}"), v)?;
                        }
                    }
                }
            }
            SchemaValidationRule::IsArray(vec) => {
                if let Some(arr) = val.as_array() {
                    for (i, el) in arr.iter().enumerate() {
                        for r in vec {
                            r.apply_rule(&format!("{key}[{i}]"), el)?;
                        }
                    }
                } else {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be an array"
                    )));
                }
            }
            SchemaValidationRule::IsUnionType(vec) => {
                let mut match_count = 0;

                for typ in vec {
                    if typ.iter().all(|r| r.apply_rule(key, val).is_ok()) {
                        match_count += 1;
                    }
                }

                if match_count != 1 {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` is not of one of the expected types"
                    )));
                }
            }
            // TODO: Implement validation on these
            SchemaValidationRule::IsDatetime => (),
            SchemaValidationRule::IsEmail => (),
            SchemaValidationRule::IsBytes => (),
        }

        Ok(())
    }
}

fn find_annotation(comm_lit: &str, annotation: &str, code: &str) -> bool {
    let a = format!("{comm_lit} {annotation}");
    for l in code.lines() {
        if !l.starts_with(comm_lit) {
            break;
        }

        if l.trim_end() == a {
            return true;
        }
    }

    false
}

pub fn should_validate_schema(code: &str, lang: &ScriptLang) -> bool {
    let annotation = "schema_validation";
    match lang {
        ScriptLang::Nativets | ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno => {
            find_annotation("//", annotation, code)
        }
        ScriptLang::Python3 => find_annotation("#", annotation, code),
        ScriptLang::Go => find_annotation("#", annotation, code),
        ScriptLang::Bash => find_annotation("#", annotation, code),
        ScriptLang::Powershell => find_annotation("#", annotation, code),
        ScriptLang::Postgresql => find_annotation("--", annotation, code),
        ScriptLang::Mysql => find_annotation("--", annotation, code),
        ScriptLang::Bigquery => find_annotation("--", annotation, code),
        ScriptLang::Snowflake => find_annotation("--", annotation, code),
        ScriptLang::Graphql => find_annotation("#", annotation, code),
        ScriptLang::Mssql => find_annotation("--", annotation, code),
        ScriptLang::OracleDB => find_annotation("--", annotation, code),
        ScriptLang::Php => find_annotation("//", annotation, code),
        ScriptLang::Rust => find_annotation("//!", annotation, code),
        ScriptLang::Ansible => find_annotation("#", annotation, code),
        ScriptLang::CSharp => find_annotation("//", annotation, code),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaValidator {
    pub required: Vec<String>,
    pub rules: Vec<(String, Vec<SchemaValidationRule>)>,
}

impl SchemaValidator {
    pub fn validate(&self, args: &HashMap<String, Box<RawValue>>) -> Result<(), Error> {
        for key in &self.required {
            if !args.contains_key(key) {
                return Err(Error::ArgumentErr(format!("Argument {key} is required")));
            }
        }

        for (key, rules) in &self.rules {
            if let Some(raw_val) = args.get(key) {
                let parsed_val = Value::from_str(raw_val.get()).map_err(|e| {
                    Error::ArgumentErr(format!("Failed to parse `{key}` argument: {e}"))
                })?;
                for rule in rules {
                    rule.apply_rule(key, &parsed_val)?;
                }
            }
        }

        Ok(())
    }

    pub fn from_schema(schema: &str) -> Result<Self, Error> {
        let schema: Value = serde_json::from_str(schema)?;

        if let Some(draft_version) = schema.get("$schema") {
            match draft_version.as_str() {
                Some("https://json-schema.org/draft/2020-12/schema") => (),
                _ => return Err(anyhow!("Supplied schema draft version is unsuported").into()),
            }
        } else {
            return Err(anyhow!("No draft version supplied").into());
        }

        let required: Vec<String> = schema
            .get("required")
            .ok_or(anyhow!("Missing `required` field on schema"))?
            .as_array()
            .ok_or(anyhow!("`required` field should be an array of strings"))?
            .into_iter()
            .map(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .ok_or(anyhow!("required field key is not a string"))
            })
            .collect::<Result<Vec<String>, anyhow::Error>>()?;

        let properties = schema
            .get("properties")
            .ok_or(anyhow!("Missing `properties` field on schema"))?
            .as_object()
            .ok_or(anyhow!("`properties` field should be an object"))?;

        let mut rules = vec![];

        for (key, val) in properties {
            rules.push((
                key.clone(),
                SchemaValidationRule::from_value(val)
                    .map_err(|e| anyhow!("Problem making rule for {key}: {e}"))?,
            ));
        }

        Ok(Self { required, rules })
    }
}

impl JsonPrimitiveType {
    fn from_str(typ: &str) -> Result<Self, anyhow::Error> {
        match typ {
            "string" => {
                return Ok(JsonPrimitiveType::String);
            }
            "number" => {
                return Ok(JsonPrimitiveType::Number);
            }
            "integer" => {
                return Ok(JsonPrimitiveType::Integer);
            }
            "object" => {
                return Ok(JsonPrimitiveType::Object);
            }
            "array" => {
                return Ok(JsonPrimitiveType::Array);
            }
            "boolean" => {
                return Ok(JsonPrimitiveType::Boolean);
            }
            "null" => {
                return Ok(JsonPrimitiveType::Null);
            }
            other => return Err(anyhow!("Received unsupported type `{other}`").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn value_to_rawvalue_map(value: Value) -> Result<HashMap<String, Box<RawValue>>, anyhow::Error> {
        match value {
            Value::Object(map) => {
                let mut result = HashMap::new();
                for (key, val) in map {
                    let raw = serde_json::to_string(&val)?; // Serialize the Value to a string
                    let raw_value: Box<RawValue> = serde_json::from_str(&raw)?; // Convert string to Box<RawValue>
                    result.insert(key, raw_value);
                }
                Ok(result)
            }
            _ => Err(anyhow!("Expected a JSON object")),
        }
    }
    #[test]
    fn test_parse_and_validate_schema() {
        let schema = r#"{
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "properties": {
        "a": {
            "contentEncoding": "base64",
            "default": null,
            "description": "",
            "originalType": "bytes",
            "type": "string"
        },
        "b": {
            "default": null,
            "description": "",
            "enum": [
                "my",
                "enum"
            ],
            "originalType": "enum",
            "type": "string"
        },
        "e": {
            "default": "inferred type string from default arg",
            "description": "",
            "originalType": "string",
            "type": "string"
        },
        "f": {
            "default": {
                "nested": "object"
            },
            "description": "",
            "properties": {
                "nested": {
                    "description": "",
                    "type": "string",
                    "originalType": "string"
                }
            },
            "type": "object"
        },
        "g": {
            "default": null,
            "description": "",
            "oneOf": [
                {
                    "type": "object",
                    "title": "Variant 1",
                    "properties": {
                        "label": {
                            "description": "",
                            "type": "string",
                            "originalType": "enum",
                            "enum": [
                                "Variant 1"
                            ]
                        },
                        "foo": {
                            "description": "",
                            "type": "string",
                            "originalType": "string"
                        }
                    }
                },
                {
                    "type": "object",
                    "title": "Variant 2",
                    "properties": {
                        "label": {
                            "description": "",
                            "type": "string",
                            "originalType": "enum",
                            "enum": [
                                "Variant 2"
                            ]
                        },
                        "bar": {
                            "description": "",
                            "type": "number"
                        }
                    }
                }
            ],
            "type": "object"
        }
    },
    "required": [
        "a",
        "b",
        "g"
    ],
    "type": "object"
}
"#;

        let validator = SchemaValidator::from_schema(schema)
            .expect("Schema couldn't be built from a valid schema");

        let args = json!(
                    {
                        "g": {
                                "label": "Variant 1",
                                "foo": ""
                        },
                        "f": {
                                "nested": "object"
                        },
                        "e": "inferred type string from default arg",
                        "b": "my",
                        "a": null
                    }
        );

        validator.validate(&value_to_rawvalue_map(args).unwrap()).err().expect("Validation should not work for this");

        let args = json!(
                    {
                        "g": {
                                "label": "Variant 1",
                                "foo": ""
                        },
                        "f": {
                                "nested": "object"
                        },
                        "e": "inferred type string from default arg",
                        "b": "not_enum",
                        "a": "123"
                    }
        );

        validator.validate(&value_to_rawvalue_map(args).unwrap()).err().expect("Validation should not work for this");

        let args = json!(
                    {
                        "g": {
                                "label": "Variant 1",
                                "foo": ""
                        },
                        "f": {
                                "nested": "object"
                        },
                        "e": "inferred type string from default arg",
                        "b": "my",
                        "a": "123"
                    }
        );

        validator.validate(&value_to_rawvalue_map(args).unwrap()).expect("Validation should work for this");
    }
}
