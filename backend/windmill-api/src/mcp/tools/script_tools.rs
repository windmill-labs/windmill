//! Script tools for MCP server
//!
//! Contains functionality for converting Windmill scripts into MCP tools.

use super::super::utils::{
    models::{ScriptInfo, ToolableItem, SchemaType},
    schema::convert_schema_to_schema_type,
    transform::transform_path,
};

/// Implementation of ToolableItem for ScriptInfo
impl ToolableItem for ScriptInfo {
    fn get_path_or_id(&self) -> String {
        transform_path(&self.path, "script")
    }

    fn get_summary(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
    }

    fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("No description")
    }

    fn get_schema(&self) -> SchemaType {
        convert_schema_to_schema_type(self.schema.clone())
    }

    fn is_hub(&self) -> bool {
        false
    }

    fn item_type(&self) -> &'static str {
        "script"
    }

    fn get_integration_type(&self) -> Option<String> {
        None
    }
}