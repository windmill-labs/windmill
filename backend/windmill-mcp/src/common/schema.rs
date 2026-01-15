/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Schema conversion utilities for MCP server
//!
//! Contains functions for converting Windmill schemas into MCP-compatible formats.

use super::types::SchemaType;
use windmill_common::scripts::Schema;

/// Convert a Windmill Schema to a SchemaType
pub fn convert_schema_to_schema_type(schema: Option<Schema>) -> SchemaType {
    let schema_obj = if let Some(ref s) = schema {
        match serde_json::from_str::<SchemaType>(s.0.get()) {
            Ok(val) => val,
            Err(_) => SchemaType::default(),
        }
    } else {
        SchemaType::default()
    };
    schema_obj
}
