use anyhow::anyhow;
use itertools::Itertools;
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
    IsOneOf(HashMap<String, Vec<SchemaValidationRule>>),
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
                    let mut rules_map: HashMap<String, Vec<SchemaValidationRule>> = HashMap::new();

                    for variant in one_of {
                        let variant_label = variant
                            .get("title")
                            .ok_or(anyhow!(
                                "oneOf variant definition should have a `title` field"
                            ))?
                            .as_str()
                            .ok_or(anyhow!(
                                "oneOf variant definition `title` field should be a string"
                            ))?;
                        if !rules_map.contains_key(variant_label) {
                            rules_map.insert(
                                variant_label.to_string(),
                                SchemaValidationRule::from_value(variant)?,
                            );
                        } else {
                            return Err(anyhow!(
                                "oneOf definition has a duplicate variant `{variant_label}`"
                            ));
                        }
                    }

                    schema_rules.push(SchemaValidationRule::IsOneOf(rules_map))
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

    fn apply_rule(&self, key: &str, val: &Value, required: bool) -> Result<(), Error> {
        if val.is_null() {
            if !required {
                return Ok(());
            }
            return Err(Error::ArgumentErr(format!("Argument {key} cannot be null")));
        }
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
                    let options = vec.iter().map(|s| s.to_string()).join(", ");
                    return Err(Error::ArgumentErr(format!(
                        "Enum type argument `{key}` expected one of `[{options}]` but received {}",
                        val.to_string()
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
                    let v = val
                        .get(&s)
                        .ok_or(Error::ArgumentErr(format!("Missing field {s} in {key}")))?;
                    for r in rules {
                        r.apply_rule(&format!("{key}.{s}"), v, true)?;
                    }
                }
            }
            SchemaValidationRule::IsArray(vec) => {
                if let Some(arr) = val.as_array() {
                    for (i, el) in arr.iter().enumerate() {
                        for r in vec {
                            r.apply_rule(&format!("{key}[{i}]"), el, true)?;
                        }
                    }
                } else {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` should be an array"
                    )));
                }
            }
            // TODO: For better error messages on OneOf, make a dedicated OneOf type that matches the label instead of trying the whole type.
            SchemaValidationRule::IsUnionType(vec) => {
                let mut match_count = 0;

                let mut errors = String::new();
                for typ in vec {
                    if let Some(e) = typ
                        .iter()
                        .map(|r| r.apply_rule(key, val, true))
                        .find_map(Result::err)
                    {
                        errors.push_str(&format!("- {e}\n"));
                    } else {
                        match_count += 1;
                    }
                }

                if match_count == 0 {
                    return Err(Error::ArgumentErr(format!(
                        "Argument `{key}` is not valid, failed matching to one of the expected types. Here is a list of possible errors:\n{errors}"
                    )));
                }
            }
            SchemaValidationRule::IsOneOf(vec) => {
                let variant_label = val
                    .get("label")
                    .ok_or(Error::ArgumentErr(format!(
                        "oneOf Variant  for argument `{key}` should have a label field"
                    )))?
                    .as_str()
                    .ok_or(Error::ArgumentErr(format!(
                        "Argument `{key}` of type oneOf expected the label to be a string"
                    )))?;

                let variant_rules = vec
                    .get(variant_label)
                    .ok_or_else(|| Error::ArgumentErr(format!(
                        "Argument `{key}` of type oneOf expected one of the following variants {}, but received `{variant_label}`", vec.keys().join(", ")
                    )))?;

                for r in variant_rules {
                    r.apply_rule(key, val, true).map_err(|e| Error::ArgumentErr(format!("Argument `{key}`: The schema for the selected oneOf variant `{variant_label}` was not respected: {e}")))?;
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
    use ScriptLang::*;
    let comment = match lang {
        Nativets | Bun | Bunnative | Deno | Php | CSharp | Java => "//",
        Python3 | Go | Bash | Powershell | Graphql | Ansible | Nu => "#",
        Postgresql | Mysql | Bigquery | Snowflake | Mssql | OracleDB | DuckDb => "--",
        Rust => "//!",
        // for related places search: ADD_NEW_LANG
    };
    find_annotation(comment, annotation, code)
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
                    rule.apply_rule(key, &parsed_val, self.required.contains(key))?;
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

    fn value_to_rawvalue_map(
        value: Value,
    ) -> Result<HashMap<String, Box<RawValue>>, anyhow::Error> {
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

        validator
            .validate(&value_to_rawvalue_map(args).unwrap())
            .err()
            .expect("Validation should not work for this");

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

        validator
            .validate(&value_to_rawvalue_map(args).unwrap())
            .err()
            .expect("Validation should not work for this");

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

        validator
            .validate(&value_to_rawvalue_map(args).unwrap())
            .expect("Validation should work for this");
    }
}
