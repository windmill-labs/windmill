use core::str;

use serde_json::value::RawValue;
use std::{collections::HashMap, str::Utf8Error};

use super::{
    converter::{Converter, ConverterError},
    replication_message::{Columns, TupleData},
};
use rust_postgres::types::Oid;
use serde_json::Value;
use windmill_common::worker::to_raw_value;
#[derive(Debug, thiserror::Error)]
pub enum RelationConversionError {
    #[error("Json error: {0}")]
    FailConversionToJson(serde_json::Error),

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

pub struct RelationConverter(HashMap<Oid, Columns>);

impl RelationConverter {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add_column(&mut self, oid: Oid, relation_body: Columns) {
        self.0.insert(oid, relation_body);
    }

    pub fn get_columns(&self, o_id: Oid) -> Result<&Columns, RelationConversionError> {
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
                TupleData::Null => to_raw_value::<&Option<()>>(&&None),
                TupleData::UnchangedToast => {
                    return Err(RelationConversionError::BinaryFormatNotSupported)
                }
                TupleData::Binary(_) => {
                    return Err(RelationConversionError::BinaryFormatNotSupported)
                }
                TupleData::Text(bytes) => {
                    let str = str::from_utf8(&bytes)?;
                    Converter::try_from_str(column.type_o_id.as_ref(), str)?
                }
            };

            object.insert(column.name.clone(), value);
        }

        Ok(object)
    }
}
