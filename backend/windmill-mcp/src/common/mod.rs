//! Common types and utilities for MCP server and client
//!
//! This module contains shared data structures, transformation utilities,
//! and scope parsing functionality used throughout the MCP implementation.

pub mod schema;
pub mod scope;
pub mod transform;
pub mod types;

pub use schema::convert_schema_to_schema_type;
pub use scope::{is_resource_allowed, parse_mcp_scopes, McpScopeConfig};
pub use transform::{
    apply_key_transformation, extract_hub_version_id_from_hashed, is_hashed_name,
    parse_hashed_name, reverse_transform, reverse_transform_key, transform_hub_path,
    transform_path,
};
pub use types::*;
