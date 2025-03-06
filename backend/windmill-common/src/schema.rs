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
            }
            JsonPrimitiveType::Number => {
                schema_rules.push(SchemaValidationRule::IsNumber);
            }
            JsonPrimitiveType::Integer => {
                schema_rules.push(SchemaValidationRule::IsInteger);
            }
            JsonPrimitiveType::Object => {
                let mut obj_rules = vec![];

                let properties = val
                    .get("properties")
                    .ok_or(anyhow!("Object type should have field `properties`"))?
                    .as_object()
                    .ok_or(anyhow!("Field properties should be an object"))?;

                for (key, v) in properties {
                    obj_rules.push((key.clone(), SchemaValidationRule::from_value(v)?))
                }

                schema_rules.push(SchemaValidationRule::IsObject(obj_rules));
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
                _ => return Err(anyhow!("Supplied scehma draft version is unsuported").into()),
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
