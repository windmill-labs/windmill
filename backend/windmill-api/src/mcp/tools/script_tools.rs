//! Script tools for MCP server
//!
//! Contains functionality for converting Windmill scripts into MCP tools.

use super::super::utils::{
    models::{SchemaType, ScriptInfo, ToolableItem},
    schema::convert_schema_to_schema_type,
};

/// Implementation of ToolableItem for ScriptInfo
impl ToolableItem for ScriptInfo {
    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn get_id(&self) -> String {
        format!("script:{}", self.hash)
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
