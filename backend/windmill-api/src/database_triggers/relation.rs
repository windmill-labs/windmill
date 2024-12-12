use core::str;

use serde_json::value::RawValue;
use std::{collections::HashMap, str::Utf8Error};

use super::{
    converter::{Converter, ConverterError},
    replication_message::{Columns, RelationBody, TupleData},
};
use rust_postgres::types::Oid;
use windmill_common::worker::to_raw_value;
#[derive(Debug, thiserror::Error)]
pub enum RelationConversionError {
    #[error("Could not find matching table")]
    FailToFindMatchingTable,

    #[error("Missing Column {0}")]
    MissingColumn(String),

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

    pub fn body_to_json(
        &self,
        to_decode: (Oid, Vec<TupleData>),
    ) -> Result<HashMap<String, Box<RawValue>>, RelationConversionError> {
        let (o_id, tuple_data) = to_decode;
        let mut object: HashMap<String, Box<RawValue>> = HashMap::new();
        let columns = self.get_columns(o_id)?;

        for (i, column) in columns.iter().enumerate() {
            let value = match &tuple_data[i] {
                TupleData::Null | TupleData::UnchangedToast => to_raw_value::<&Option<()>>(&&None),
                TupleData::Binary(_) => {
                    return Err(RelationConversionError::BinaryFormatNotSupported)
                }
                TupleData::Text(bytes) => {
                    let str = str::from_utf8(&bytes[..])?;
                    Converter::try_from_str(column.type_o_id.as_ref(), str)?
                }
            };

            object.insert(column.name.clone(), value);
        }
        Ok(HashMap::from([("row".to_string(), to_raw_value(&object))]))
    }
}
