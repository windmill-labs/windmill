//! Flow tools for MCP server
//!
//! Contains functionality for converting Windmill flows into MCP tools.

use super::super::utils::{
    models::{FlowInfo, SchemaType, ToolableItem},
    schema::convert_schema_to_schema_type,
};

/// Implementation of ToolableItem for FlowInfo
impl ToolableItem for FlowInfo {
    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn get_id(&self) -> String {
        format!("flow:{}", self.id)
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
        "flow"
    }

    fn get_integration_type(&self) -> Option<String> {
        None
    }
}
