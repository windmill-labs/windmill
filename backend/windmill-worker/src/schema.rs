use std::collections::HashMap;
use windmill_common::schema::{SchemaValidationRule, SchemaValidator};
use windmill_parser::{MainArgSignature, Typ};

fn make_rules_for_arg_typ(typ: &Typ) -> Vec<SchemaValidationRule> {
    let mut rules = vec![];

    match typ {
        Typ::Str(enum_variants) => {
            rules.push(SchemaValidationRule::IsString);

            if let Some(enum_variants) = enum_variants {
                rules.push(SchemaValidationRule::StrictEnum(
                    enum_variants
                        .iter()
                        .map(|v| serde_json::Value::String(v.to_string()))
                        .collect(),
                ));
            }
        }
        Typ::Int => {
            rules.push(SchemaValidationRule::IsInteger);
        }
        Typ::Float => {
            rules.push(SchemaValidationRule::IsNumber);
        }
        Typ::Bool => {
            rules.push(SchemaValidationRule::IsBool);
        }
        Typ::List(typ) => {
            rules.push(SchemaValidationRule::IsArray(make_rules_for_arg_typ(typ)));
        }
        Typ::Bytes => {
            rules.push(SchemaValidationRule::IsString);
            rules.push(SchemaValidationRule::IsBytes);
        }
        Typ::Datetime | Typ::Date => {
            rules.push(SchemaValidationRule::IsString);
            rules.push(SchemaValidationRule::IsDatetime);
        }
        Typ::Email => {
            rules.push(SchemaValidationRule::IsString);
            rules.push(SchemaValidationRule::IsEmail);
        }
        Typ::Sql => {
            rules.push(SchemaValidationRule::IsString);
        }
        Typ::Object(object_type) => {
            let mut obj_rules = vec![];

            if let Some(props) = &object_type.props {
                for prop in props {
                    obj_rules.push((prop.key.to_string(), make_rules_for_arg_typ(&prop.typ)));
                }
            }

            rules.push(SchemaValidationRule::IsObject(obj_rules))
        }
        Typ::OneOf(variants) => {
            let mut rules_map = HashMap::new();

            for variant in variants {
                let mut obj_rules = vec![];

                for prop in &variant.properties {
                    obj_rules.push((prop.key.to_string(), make_rules_for_arg_typ(&prop.typ)));
                }
                rules_map.insert(
                    variant.label.to_string(),
                    vec![SchemaValidationRule::IsObject(obj_rules)],
                );
            }

            rules.push(SchemaValidationRule::IsOneOf(rules_map))
        }
        Typ::Resource(_) => (),
        Typ::DynSelect(_) => (),
        Typ::DynMultiselect(_) => (),
        Typ::Unknown => (),
    }

    rules
}

pub fn schema_validator_from_main_arg_sig(sig: &MainArgSignature) -> SchemaValidator {
    let mut rules = vec![];
    let mut required = vec![];

    for arg in &sig.args {
        if !arg.has_default {
            required.push(arg.name.to_string());
        }

        rules.push((arg.name.to_string(), make_rules_for_arg_typ(&arg.typ)));
    }

    SchemaValidator { required, rules }
}
