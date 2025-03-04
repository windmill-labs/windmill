use anyhow::anyhow;
use serde_json::{value::RawValue, Value};
use sqlx::types::Json;
use windmill_common::scripts::ScriptLang;
use std::collections::HashMap;
use windmill_parser::{Arg, MainArgSignature};

enum SchemaError {
    MissingArg(String),
    UnspecifiedArg(String),
    WrongShapeForArg(String),
    InvalidArg((String, String)),
}

// fn validate_typ(arg_schema: &Arg, arg: &Box<RawValue>) -> Result<String, SchemaError> {
//     let logs: String;
//     match arg_schema.typ.as_ref() {
//         windmill_parser::Typ::Str(vec) => {
//             let raw_string = arg.get();
//             if !raw_string.starts_with("\"") || !raw_string.ends_with("\"") {
//                 return Err(SchemaError::InvalidArg((arg_schema.name.clone(), "string".to_string())));
//             }
//             Ok("".to_string())
//         }
//         windmill_parser::Typ::Int => {
//             // serde_json::from_str(arg.get());
//             todo!()
//         },
//         windmill_parser::Typ::Float => todo!(),
//         windmill_parser::Typ::Bool => todo!(),
//         windmill_parser::Typ::List(typ) => todo!(),
//         windmill_parser::Typ::Bytes => todo!(),
//         windmill_parser::Typ::Datetime => todo!(),
//         windmill_parser::Typ::Resource(_) => todo!(),
//         windmill_parser::Typ::Email => todo!(),
//         windmill_parser::Typ::Sql => todo!(),
//         windmill_parser::Typ::Object(vec) => todo!(),
//         windmill_parser::Typ::OneOf(vec) => todo!(),
//         windmill_parser::Typ::DynSelect(_) => {
//             Ok(format!("Warning: argument {} of type DynSelect skipped schema validation as it is not supported for this argument type", &arg_schema.name))
//         }
//         windmill_parser::Typ::Unknown => {
//             Ok(format!("Warning: argument {} of type Unknown skipped schema validation as it is not supported for this argument type", &arg_schema.name))
//         }
//     }
// }

// fn fake_validate_schema(
//     args: Json<HashMap<String, Box<RawValue>>>,
//     schema: MainArgSignature,
// ) -> Result<String, SchemaError> {
//     let mut logs = String::new();
//
//     for arg_schema in schema.args {
//         let arg = args
//             .get(&arg_schema.name)
//             .ok_or(SchemaError::MissingArg(arg_schema.name.clone()))?;
//     }
//
//     Ok(logs)
// }


// pub trait ValidatesSchema: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + Send + Sync {
//     fn validate(
//         &self,
//         instance: Option<&Json<HashMap<String, Box<RawValue>>>>,
//     ) -> anyhow::Result<()>;
// }

pub enum SchemaValidator {
    MainArgValidator(MainArgSchemaValidator),
    JsonValidator(JsonSchemaValidator)
}

struct MainArgSchemaValidator {
    main_arg_sig: MainArgSignature,
}

struct JsonSchemaValidator {
    validator: jsonschema::Validator,
}

// impl ValidatesSchema for JsonSchemaValidator {
//     fn validate(
//         &self,
//         instance: Option<&Json<HashMap<String, Box<RawValue>>>>,
//     ) -> anyhow::Result<()> {
//         if let Some(raw_map) = instance {
//             let parsed_map: HashMap<String, Value> = raw_map
//                 .iter()
//                 .map(|(k, v)| (k.to_string(), serde_json::from_str(v.get()).unwrap()))
//                 .collect();
//
//             // Convert HashMap into a serde_json::Value
//             let value = Value::Object(parsed_map.into_iter().collect());
//
//             if let Err(e) = self.validator.validate(&value) {
//                 return Err(anyhow!("{}", e));
//             }
//         } else {
//             return Err(anyhow!("Arguments are missing"));
//         }
//
//         Ok(())
//     }
// }

// pub struct SchemaValidator {
//     pub rules: Vec<SchemaValidationRule>
//
// }
//
// enum SchemaValidationRule {
//
// }
//
// impl SchemaValidator {
//     fn from_schema(schema: Box<RawValue>) -> Result<SchemaValidator> {
//         let val = serde_json::from_str(schema.get())?;
//         SchemaValidator { rules: todo!() }
//
//     }
// }

// pub fn make_schema_validator(schema: &Box<RawValue>) -> anyhow::Result<impl ValidatesSchema> {
//     let schema: serde_json::Value = serde_json::from_str(schema.get())?;
//     let validator = jsonschema::validator_for(&schema)?;
//
//     Ok(JsonSchemaValidator { validator })
// }
//
// pub fn make_main_arg_validator(code: &str, language: Option<ScriptLang>) -> anyhow::Result<impl ValidatesSchema> {
//
// }

// pub fn validate_schema(
//     args: Json<HashMap<String, Box<RawValue>>>,
//     schema_validator: impl ValidatesSchema,
// ) -> Result<String, SchemaError> {
//     let mut logs = String::new();
//
//     for arg_schema in schema.args {
//         let arg = args
//             .get(&arg_schema.name)
//             .ok_or(SchemaError::MissingArg(arg_schema.name.clone()))?;
//     }
//
//     Ok(logs)
// }
