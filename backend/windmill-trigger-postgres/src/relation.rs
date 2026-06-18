use core::str;

use serde_json::{Map, Value};
use std::{collections::HashMap, str::Utf8Error};

use super::{
    converter::{Converter, ConverterError},
    replication_message::{Columns, RelationBody, TupleData},
};
use rust_postgres::types::Oid;
#[derive(Debug, thiserror::Error)]
pub enum RelationConversionError {
    #[error("Could not find matching table")]
    FailToFindMatchingTable,

    #[error("Binary data not supported")]
    BinaryFormatNotSupported,

    #[error("decode error: {0}")]
    FromBytes(#[from] ConverterError),

    #[error("invalid string value")]
    InvalidStr(#[from] Utf8Error),
}

pub struct RelationConverter(HashMap<Oid, RelationBody>);

impl RelationConverter {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add_relation(&mut self, relation: RelationBody) {
        self.0.insert(relation.o_id, relation);
    }

    pub fn get_columns(&self, o_id: Oid) -> Result<&Columns, RelationConversionError> {
        self.0
            .get(&o_id)
            .map(|relation_body| &relation_body.columns)
            .ok_or(RelationConversionError::FailToFindMatchingTable)
    }

    pub fn get_relation(&self, o_id: Oid) -> Result<&RelationBody, RelationConversionError> {
        self.0
            .get(&o_id)
            .ok_or(RelationConversionError::FailToFindMatchingTable)
    }

    pub fn row_to_json(
        &self,
        to_decode: (Oid, Vec<TupleData>),
    ) -> Result<Map<String, Value>, RelationConversionError> {
        let (o_id, tuple_data) = to_decode;
        let mut object: Map<String, Value> = Map::new();
        let columns = self.get_columns(o_id)?;

        for (i, column) in columns.iter().enumerate() {
            let value = match &tuple_data[i] {
                TupleData::Null | TupleData::UnchangedToast => Value::Null,
                TupleData::Binary(_) => {
                    return Err(RelationConversionError::BinaryFormatNotSupported)
                }
                TupleData::Text(bytes) => {
                    let str = str::from_utf8(&bytes[..])?;
                    Converter::try_from_str(column.type_o_id.clone(), str)?
                }
            };

            object.insert(column.name.clone(), value);
        }
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use rust_postgres::types::Type;
    use serde_json::json;

    use super::super::replication_message::{Column, RelationBody, ReplicaIdentity};

    fn make_relation(o_id: Oid, columns: Vec<Column>) -> RelationBody {
        RelationBody::new(
            None,
            o_id,
            "public".to_string(),
            "test".to_string(),
            ReplicaIdentity::Default,
            columns,
        )
    }

    fn text_col(name: &str, typ: Option<Type>) -> Column {
        Column::new(0, name.to_string(), typ, -1)
    }

    #[test]
    fn test_row_to_json_text_columns() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(
            1,
            vec![
                text_col("id", Some(Type::INT4)),
                text_col("name", Some(Type::TEXT)),
            ],
        ));

        let tuple = vec![
            TupleData::Text(Bytes::from("42")),
            TupleData::Text(Bytes::from("Alice")),
        ];

        let result = converter.row_to_json((1, tuple)).unwrap();
        assert_eq!(result["id"], json!(42));
        assert_eq!(result["name"], json!("Alice"));
    }

    #[test]
    fn test_row_to_json_with_null() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(
            1,
            vec![
                text_col("id", Some(Type::INT4)),
                text_col("email", Some(Type::TEXT)),
            ],
        ));

        let tuple = vec![TupleData::Text(Bytes::from("1")), TupleData::Null];

        let result = converter.row_to_json((1, tuple)).unwrap();
        assert_eq!(result["id"], json!(1));
        assert_eq!(result["email"], Value::Null);
    }

    #[test]
    fn test_row_to_json_with_unchanged_toast() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(1, vec![text_col("data", Some(Type::TEXT))]));

        let tuple = vec![TupleData::UnchangedToast];
        let result = converter.row_to_json((1, tuple)).unwrap();
        assert_eq!(result["data"], Value::Null);
    }

    #[test]
    fn test_row_to_json_binary_rejected() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(1, vec![text_col("data", Some(Type::BYTEA))]));

        let tuple = vec![TupleData::Binary(Bytes::from("data"))];
        assert!(matches!(
            converter.row_to_json((1, tuple)),
            Err(RelationConversionError::BinaryFormatNotSupported)
        ));
    }

    #[test]
    fn test_missing_relation() {
        let converter = RelationConverter::new();
        let tuple = vec![TupleData::Null];
        assert!(matches!(
            converter.row_to_json((999, tuple)),
            Err(RelationConversionError::FailToFindMatchingTable)
        ));
    }

    #[test]
    fn test_multiple_relations() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(1, vec![text_col("a", Some(Type::TEXT))]));
        converter.add_relation(make_relation(2, vec![text_col("b", Some(Type::INT4))]));

        let result1 = converter
            .row_to_json((1, vec![TupleData::Text(Bytes::from("hello"))]))
            .unwrap();
        let result2 = converter
            .row_to_json((2, vec![TupleData::Text(Bytes::from("42"))]))
            .unwrap();

        assert_eq!(result1["a"], json!("hello"));
        assert_eq!(result2["b"], json!(42));
    }

    #[test]
    fn test_row_to_json_bool_column() {
        let mut converter = RelationConverter::new();
        converter.add_relation(make_relation(1, vec![text_col("active", Some(Type::BOOL))]));

        let result = converter
            .row_to_json((1, vec![TupleData::Text(Bytes::from("t"))]))
            .unwrap();
        assert_eq!(result["active"], json!(true));
    }
}
