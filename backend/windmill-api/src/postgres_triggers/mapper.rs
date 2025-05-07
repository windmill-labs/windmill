use std::collections::HashMap;

use rust_postgres::types::Type;

use super::handler::Language;

fn postgres_to_typescript_type(postgres_type: Option<Type>) -> String {
    let data_type = match postgres_type {
        Some(postgres_type) => match postgres_type {
            Type::BOOL => "boolean",
            Type::BOOL_ARRAY => "Array<boolean>",
            Type::CHAR | Type::BPCHAR | Type::VARCHAR | Type::NAME | Type::TEXT => "string",
            Type::CHAR_ARRAY
            | Type::BPCHAR_ARRAY
            | Type::VARCHAR_ARRAY
            | Type::NAME_ARRAY
            | Type::TEXT_ARRAY => "Array<string>",
            Type::INT2 | Type::INT4 | Type::INT8 | Type::NUMERIC => "number",
            Type::INT2_ARRAY | Type::INT4_ARRAY | Type::INT8_ARRAY => "Array<number>",
            Type::FLOAT4 | Type::FLOAT8 => "number",
            Type::FLOAT8_ARRAY | Type::FLOAT4_ARRAY => "Array<number>",
            Type::NUMERIC_ARRAY => "Array<number>",
            Type::BYTEA => "Array<number>",
            Type::BYTEA_ARRAY => "Array<Array<number>>",
            Type::DATE => "string",
            Type::DATE_ARRAY => "Array<string>",
            Type::TIME => "string",
            Type::TIME_ARRAY => "Array<string>",
            Type::TIMESTAMPTZ | Type::TIMESTAMP => "string",
            Type::TIMESTAMPTZ_ARRAY | Type::TIMESTAMP_ARRAY => "Array<string>",
            Type::UUID => "string",
            Type::UUID_ARRAY => "Array<string>",
            Type::JSON | Type::JSONB | Type::JSON_ARRAY | Type::JSONB_ARRAY => "unknown",
            Type::OID => "number",
            Type::OID_ARRAY => "Array<number>",
            _ => "string",
        },
        None => "string",
    };

    data_type.to_string()
}

fn into_body_struct(language: Language, mapped_info: Vec<MappingInfo>) -> String {
    let mut block = String::new();
    match language {
        Language::Typescript => {
            block.push_str("{\r\n");
            for field in mapped_info {
                let typescript_type = postgres_to_typescript_type(field.data_type);
                let mut key = field.column_name;
                if field.is_nullable {
                    key.push('?');
                }
                let full_field = format!("\t\t{}: {},\r\n", key, typescript_type);
                block.push_str(&full_field);
            }
            block.push_str("\t}");
        }
    }
    block
}

#[derive(Debug)]
pub struct MappingInfo {
    data_type: Option<Type>,
    is_nullable: bool,
    column_name: String,
}

impl MappingInfo {
    pub fn new(column_name: String, data_type: Option<Type>, is_nullable: bool) -> Self {
        Self { column_name, data_type, is_nullable }
    }
}

pub struct Mapper {
    to_template: HashMap<String, HashMap<String, Vec<MappingInfo>>>,
    language: Language,
}

impl Mapper {
    pub fn new(
        to_template: HashMap<String, HashMap<String, Vec<MappingInfo>>>,
        language: Language,
    ) -> Self {
        Self { to_template, language }
    }

    fn into_typescript_template(self) -> Vec<String> {
        let mut struct_definitions = Vec::new();
        for (_, mapping_info) in self.to_template {
            let last_elem = mapping_info.len() - 1;
            for (i, (_, mapped_info)) in mapping_info.into_iter().enumerate() {
                let mut struct_body = into_body_struct(Language::Typescript, mapped_info);
                let struct_body = if i != last_elem {
                    struct_body.push_str("\r\n");
                    struct_body
                } else {
                    struct_body
                };
                struct_definitions.push(struct_body);
            }
        }
        struct_definitions
    }

    pub fn get_template(self) -> String {
        let struct_definition = match self.language {
            Language::Typescript => self.into_typescript_template(),
        };

        let struct_definition = if struct_definition.is_empty() {
            "any".to_string()
        } else {
            struct_definition.join("\t| ")
        };

        format!(
            r#"


export async function main(
  transaction_type: "insert" | "update" | "delete",
  schema_name: string,
  table_name: string,
  row: {},
  old_row?: {}
) {{
}}
    "#,
            &struct_definition,
            &struct_definition
        )
    }
}
