//! Hub tools for MCP server
//!
//! Contains functionality for integrating Windmill Hub scripts as MCP tools.

use super::super::utils::{
    models::{HubScriptInfo, ToolableItem, SchemaType},
};

/// Implementation of ToolableItem for HubScriptInfo
impl ToolableItem for HubScriptInfo {
    fn get_path_or_id(&self) -> String {
        let id = self.version_id;
        let summary = self.summary.as_deref().unwrap_or("No summary");
        format!("hs-{}-{}", id, summary.replace(" ", "_"))
    }

    fn get_summary(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
    }

    fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("No description")
    }

    fn get_schema(&self) -> SchemaType {
        match serde_json::from_value::<SchemaType>(self.schema.clone().unwrap_or_default()) {
            Ok(schema_type) => schema_type,
            Err(_) => SchemaType::default(),
        }
    }

    fn is_hub(&self) -> bool {
        true
    }

    fn item_type(&self) -> &'static str {
        "script"
    }

    fn get_integration_type(&self) -> Option<String> {
        self.app.clone()
    }
}