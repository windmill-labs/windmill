use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use axum::body::{to_bytes};
use axum::Router;
use axum::{extract::Path, http::Request, middleware::Next, response::Response};
use rmcp::{
    handler::server::ServerHandler,
    model::*,
    service::{RequestContext, RoleServer},
    Error,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sql_builder::prelude::*;
use sqlx::FromRow;
use tokio::try_join;
use windmill_common::auth::create_jwt_token;
use windmill_common::db::{Authed, UserDB};
use windmill_common::worker::to_raw_value;
use windmill_common::{DB, HUB_BASE_URL, BASE_URL};

use windmill_common::scripts::{get_full_hub_script_by_path, Schema};

use crate::db::ApiAuthed;
use crate::jobs::{
    run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
};
use crate::HTTP_CLIENT;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, SessionManager, StreamableHttpService,
};
use windmill_common::utils::{query_elems_from_hub, StripPath};

use crate::mcp_tools::{all_tools, EndpointTool};

pub fn endpoint_tools_to_mcp_tools(endpoint_tools: Vec<EndpointTool>) -> Vec<Tool> {
    endpoint_tools.into_iter().map(|tool| endpoint_tool_to_mcp_tool(&tool)).collect()
}

pub fn endpoint_tool_to_mcp_tool(tool: &EndpointTool) -> Tool {
    // Combine all schemas into a single input schema
    let mut combined_properties = serde_json::Map::new();
    let mut combined_required = Vec::new();
    
    // Add path parameters
    if let Some(path_schema) = &tool.path_params_schema {
        if let Some(props) = path_schema.get("properties").and_then(|p| p.as_object()) {
            for (key, value) in props {
                combined_properties.insert(key.clone(), value.clone());
            }
        }
        if let Some(required) = path_schema.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(req_str) = req.as_str() {
                    combined_required.push(req_str.to_string());
                }
            }
        }
    }
    
    // Add query parameters
    if let Some(query_schema) = &tool.query_params_schema {
        if let Some(props) = query_schema.get("properties").and_then(|p| p.as_object()) {
            for (key, value) in props {
                combined_properties.insert(key.clone(), value.clone());
            }
        }
        if let Some(required) = query_schema.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(req_str) = req.as_str() {
                    combined_required.push(req_str.to_string());
                }
            }
        }
    }
    
    // Add body parameters
    if let Some(body_schema) = &tool.body_schema {
        if let Some(props) = body_schema.get("properties").and_then(|p| p.as_object()) {
            for (key, value) in props {
                combined_properties.insert(key.clone(), value.clone());
            }
        }
        if let Some(required) = body_schema.get("required").and_then(|r| r.as_array()) {
            for req in required {
                if let Some(req_str) = req.as_str() {
                    combined_required.push(req_str.to_string());
                }
            }
        }
    }
    
    let combined_schema = serde_json::json!({
        "type": "object",
        "properties": combined_properties,
        "required": combined_required
    });
    
    Tool {
        name: tool.name.clone(),
        description: Some(tool.description.clone()),
        input_schema: Arc::new(combined_schema.as_object().unwrap().clone()),
        annotations: Some(ToolAnnotations {
            title: Some(format!("{} {}", 
                match tool.method {
                    http::Method::GET => "GET",
                    http::Method::POST => "POST", 
                    http::Method::PUT => "PUT",
                    http::Method::DELETE => "DELETE",
                    http::Method::PATCH => "PATCH",
                    _ => "UNKNOWN"
                }, 
                tool.path
            )),
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        }),
    }
}

/// Transforms the path for workspace scripts/flows.
///
/// This function takes a path and a type string.
/// It then formats the transformed path with the type prefix.
/// This is used when listing, because we can't have names with slashes.
/// Because we replace slashes with underscores, we also need to escape underscores.
///
/// # Parameters
/// - `path`: The path to transform.
/// - `type_str`: The type of the item (script or flow).
///
/// # Returns
/// - `String`: The transformed path.
fn transform_path(path: &str, type_str: &str) -> String {
    // Only apply special underscore escaping for paths starting with "f/"
    let transformed = if path.starts_with("f/") {
        let escaped_path = path.replace('_', "__");
        escaped_path.replace('/', "_")
    } else {
        path.replace('/', "_")
    };

    // first letter of type_str is used as prefix, only one letter to avoid reaching 60 char name limit
    format!("{}-{}", &type_str[..1], transformed)
}

fn convert_schema_to_schema_type(schema: Option<Schema>) -> SchemaType {
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

trait ToolableItem {
    fn get_path_or_id(&self) -> String;
    fn get_summary(&self) -> &str;
    fn get_description(&self) -> &str;
    fn get_schema(&self) -> SchemaType;
    fn is_hub(&self) -> bool;
    fn item_type(&self) -> &'static str;
    fn get_integration_type(&self) -> Option<String>;
}

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

impl ToolableItem for FlowInfo {
    fn get_path_or_id(&self) -> String {
        transform_path(&self.path, "flow")
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

#[derive(Clone)]
pub struct Runner {}

#[derive(Serialize, Deserialize, Debug)]
struct HubResponse {
    asks: Vec<HubScriptInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HubScriptInfo {
    version_id: u64,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Value>,
    app: Option<String>,
}

#[derive(Serialize, FromRow, Deserialize, Debug, Clone)]
struct SchemaType {
    r#type: String,
    properties: std::collections::HashMap<String, serde_json::Value>,
    required: Vec<String>,
}

impl Default for SchemaType {
    fn default() -> Self {
        Self {
            r#type: "object".to_string(),
            properties: std::collections::HashMap::new(),
            required: vec![],
        }
    }
}

#[derive(Serialize, FromRow, Debug)]
struct ScriptInfo {
    path: String,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Schema>,
}

#[derive(Serialize, FromRow)]
struct ItemSchema {
    schema: Option<Schema>,
}

#[derive(Serialize, FromRow, Debug)]
struct FlowInfo {
    path: String,
    summary: Option<String>,
    description: Option<String>,
    schema: Option<Schema>,
}

#[derive(Serialize, FromRow, Debug)]
struct ResourceInfo {
    path: String,
    description: Option<String>,
    resource_type: String,
}

#[derive(Serialize, FromRow, Debug, Clone)]
struct ResourceType {
    name: String,
    description: Option<String>,
}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    fn check_scopes(authed: &ApiAuthed) -> Result<(), Error> {
        let scopes = authed.scopes.as_ref();
        if scopes.is_none()
            || scopes
                .unwrap()
                .iter()
                .all(|scope| scope != "mcp:all" && scope != "mcp:favorites" && !scope.starts_with("mcp:hub:"))
        {
            tracing::error!("Unauthorized: missing mcp scope");
            return Err(Error::internal_error("Unauthorized: missing mcp scope".to_string(), None));
        }
        Ok(())
    }

    async fn get_item_schema(
        path: &str,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        item_type: &str,
    ) -> Result<Option<Schema>, Error> {
        let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
        sqlb.fields(&["o.schema"]);
        sqlb.and_where("o.path = ?".bind(&path));
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        sqlb.and_where("o.archived = false");
        sqlb.and_where("o.draft_only IS NOT TRUE");
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let item = sqlx::query_as::<_, ItemSchema>(&sql)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("failed to fetch item schema: {}", _e);
                Error::internal_error("failed to fetch item schema", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(item.schema)
    }

    /// Reverses the transformation of a path.
    ///
    /// This function takes a transformed path and reverses the transformation applied by `transform_path`.
    /// It checks if the path starts with "h" (indicating a Hub script) and removes the prefix if present.
    /// It then determines the type of the item (script or flow) based on the prefix.
    /// This is used in call_tool to get the original path, and the type of the item.
    ///
    /// # Parameters
    /// - `transformed_path`: The transformed path to reverse.
    ///
    /// # Returns
    /// - `Result<(&str, String, bool), String>`: A tuple containing the original path, the type of the item, and a boolean indicating if it's a Hub script.
    /// - `Err(String)`: If the path is invalid.
    fn reverse_transform(transformed_path: &str) -> Result<(&str, String, bool), String> {
        let is_hub = transformed_path.starts_with("h");
        let transformed_path = if is_hub {
            transformed_path[1..].to_string()
        } else {
            transformed_path.to_string()
        };
        let type_str = if transformed_path.starts_with("s-") {
            "script"
        } else if transformed_path.starts_with("f-") {
            "flow"
        } else {
            return Err(format!(
                "Invalid prefix in transformed path: {}",
                transformed_path
            ));
        };

        let mangled_path = &transformed_path[2..];

        // Check if this path was previously transformed with special underscore handling
        let is_special_path = mangled_path.starts_with("f_");

        let original_path = if is_hub {
            let parts = mangled_path.split("-").collect::<Vec<&str>>();
            parts[0].to_string()
        } else if is_special_path {
            const TEMP_PLACEHOLDER: &str = "@@UNDERSCORE@@";
            let path_with_placeholder = mangled_path.replace("__", TEMP_PLACEHOLDER);
            let path_with_slashes = path_with_placeholder.replace('_', "/");
            path_with_slashes.replace(TEMP_PLACEHOLDER, "_")
        } else {
            mangled_path.replacen('_', "/", 2)
        };

        Ok((type_str, original_path, is_hub))
    }

    async fn inner_get_resources_types(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
    ) -> Result<Vec<ResourceType>, Error> {
        let mut sqlb = SqlBuilder::select_from("resource_type as o");
        sqlb.fields(&["o.name", "o.description"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ResourceType>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch resource types: {}", _e);
                Error::internal_error("failed to fetch resource types", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    async fn inner_get_resources(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        resource_type: &str,
    ) -> Result<Vec<ResourceInfo>, Error> {
        let mut sqlb = SqlBuilder::select_from("resource as o");
        sqlb.fields(&["o.path", "o.description", "o.resource_type"]);
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
        sqlb.and_where("o.resource_type = ?".bind(&resource_type));
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, ResourceInfo>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch resources: {}", _e);
                Error::internal_error("failed to fetch resources", None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;

        Ok(rows)
    }

    async fn inner_get_items<T: for<'a> sqlx::FromRow<'a, sqlx::postgres::PgRow> + Send + Unpin>(
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        scope_type: &str,
        item_type: &str,
    ) -> Result<Vec<T>, Error> {
        let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
        let fields = vec!["o.path", "o.summary", "o.description", "o.schema"];
        sqlb.fields(&fields);
        if scope_type == "favorites" {
            sqlb.join("favorite")
                .on("favorite.favorite_kind = ? AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?".bind(&item_type)
                    .bind(&authed.username));
        }
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
            .and_where("o.archived = false")
            .and_where("o.draft_only IS NOT TRUE");

        if item_type == "script" {
            sqlb.and_where("(o.no_main_func IS NOT TRUE OR o.no_main_func IS NULL)");
        }

        sqlb.order_by(
            if item_type == "flow" {
                "o.edited_at"
            } else {
                "o.created_at"
            },
            false,
        )
        .limit(100);
        let sql = sqlb.sql().map_err(|_e| {
            tracing::error!("failed to build sql: {}", _e);
            Error::internal_error("failed to build sql", None)
        })?;
        let mut tx = user_db
            .clone()
            .begin(authed)
            .await
            .map_err(|_e| Error::internal_error("failed to begin transaction", None))?;
        let rows = sqlx::query_as::<_, T>(&sql)
            .fetch_all(&mut *tx)
            .await
            .map_err(|_e| {
                tracing::error!("Failed to fetch {}: {}", item_type, _e);
                Error::internal_error(format!("failed to fetch {}", item_type), None)
            })?;
        tx.commit()
            .await
            .map_err(|_e| Error::internal_error("failed to commit transaction", None))?;
        Ok(rows)
    }

    async fn inner_get_scripts_from_hub(
        db: &DB,
        scope_integrations: Option<&str>,
    ) -> Result<Vec<HubScriptInfo>, Error> {
        let query_params = Some(vec![
            ("limit", "100".to_string()),
            ("with_schema", "true".to_string()),
            ("apps", scope_integrations.unwrap_or("").to_string()),
        ]);
        let url = format!("{}/scripts/top", *HUB_BASE_URL.read().await);
        let (_status_code, _headers, response) =
            query_elems_from_hub(&HTTP_CLIENT, &url, query_params, &db)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get items from hub: {}", e);
                    Error::internal_error(format!("Failed to get items from hub: {}", e), None)
                })?;
        let body_bytes = to_bytes(response, usize::MAX).await.map_err(|e| {
            tracing::error!("Failed to read response body: {}", e);
            Error::internal_error(format!("Failed to read response body: {}", e), None)
        })?;
        let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
            tracing::error!("Failed to decode response body: {}", e);
            Error::internal_error(format!("Failed to decode response body: {}", e), None)
        })?;
        let hub_response: HubResponse = serde_json::from_str(&body_str).map_err(|e| {
            tracing::error!("Failed to parse hub response: {}", e);
            Error::internal_error(format!("Failed to parse hub response: {}", e), None)
        })?;

        Ok(hub_response.asks)
    }

    /// Reverses the transformation of a key.
    ///
    /// This function takes a transformed key and a schema object.
    /// It then reverses the transformation applied by `apply_key_transformation`. This can be subject to collisions, but it's unlikely and is ok for our use case.
    /// # Parameters
    /// - `transformed_key`: The transformed key to reverse.
    /// - `schema_obj`: The schema object.
    ///
    /// # Returns
    /// - `String`: The original key.
    fn reverse_transform_key(transformed_key: &str, schema_obj: &Option<SchemaType>) -> String {
        let schema_obj = match schema_obj {
            Some(s) => s,
            None => {
                // No schema available, return the key as is (best guess)
                return transformed_key.to_string();
            }
        };

        for original_key_in_schema in schema_obj.properties.keys() {
            // Apply the SAME forward transformation to the schema key
            let potential_transformed_key =
                Runner::apply_key_transformation(original_key_in_schema);

            // If it matches the key we received, we found the likely original
            if potential_transformed_key == transformed_key {
                return original_key_in_schema.clone();
            }
        }

        transformed_key.to_string()
    }

    /// Applies a key transformation to a key.
    ///
    /// This function takes a key and replaces spaces with underscores.
    /// It also removes any characters that are not alphanumeric or underscores.
    /// This is used when listing, because we can't have names with spaces or special characters in the schema properties.
    /// # Parameters
    /// - `key`: The key to transform.
    ///
    /// # Returns
    /// - `String`: The transformed key.
    fn apply_key_transformation(key: &str) -> String {
        key.replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
    }

    /// Transforms the schema for resources.
    ///
    /// This function takes a schema and a database connection, and attempts to transform the schema for resources.
    /// It replaces invalid characters in property keys with underscores and converts object properties to strings.
    /// It also fetches resource type information and adds it to the description of resource properties.
    ///
    /// # Parameters
    /// - `schema`: The schema to transform.
    /// - `user_db`: The database connection.
    /// - `authed`: The authenticated user.
    /// - `w_id`: The workspace ID.
    /// - `resources_cache`: A mutable reference to the resources cache.
    /// - `resources_types`: A reference to the resource types.
    ///
    /// # Returns
    /// - `Result<SchemaType, Error>`: The transformed schema.
    /// - `Err(Error)`: If the transformation fails.
    async fn transform_schema_for_resources(
        schema: &SchemaType,
        user_db: &UserDB,
        authed: &ApiAuthed,
        w_id: &str,
        resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
        resources_types: &Vec<ResourceType>,
    ) -> Result<SchemaType, Error> {
        let mut schema_obj: SchemaType = schema.clone();

        // replace invalid char in property key with underscore
        let replacements: Vec<(String, String, serde_json::Value)> = schema_obj
            .properties
            .iter()
            .filter_map(|(key, value)| {
                if key.chars().any(|c| !c.is_alphanumeric() && c != '_') {
                    let new_key = Runner::apply_key_transformation(key);
                    Some((key.clone(), new_key, value.clone()))
                } else {
                    None
                }
            })
            .collect();

        for (old_key, new_key, value) in replacements {
            schema_obj.properties.remove(&old_key);
            schema_obj.properties.insert(new_key, value);
        }

        for (_key, prop_value) in schema_obj.properties.iter_mut() {
            if let serde_json::Value::Object(prop_map) = prop_value {
                // if property is a resource, fetch the resource type infos, and add each available resource to the description
                if let Some(format_value) = prop_map.get("format") {
                    if let serde_json::Value::String(format_str) = format_value {
                        if format_str.starts_with("resource-") {
                            let resource_type_key =
                                format_str.split("-").last().unwrap_or_default().to_string();
                            let resource_type = resources_types
                                .iter()
                                .find(|rt| rt.name == resource_type_key);
                            let resource_type_obj = resource_type.cloned();

                            if !resources_cache.contains_key(&resource_type_key) {
                                let available_resources = Runner::inner_get_resources(
                                    user_db,
                                    authed,
                                    &w_id,
                                    &resource_type_key,
                                )
                                .await;

                                match available_resources {
                                    Ok(cache_data) => {
                                        resources_cache
                                            .insert(resource_type_key.clone(), cache_data);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to fetch resource cache data: {}",
                                            e
                                        );
                                        continue; // Skip this property if fetching failed
                                    }
                                }
                            }

                            if let Some(resource_cache) = resources_cache.get(&resource_type_key) {
                                let resources_count = resource_cache.len();
                                let description = match resource_type_obj {
                                    Some(resource_type_obj) => format!(
                                        "This is a resource named `{}` with the following description: `{}`.\nThe path of the resource should be used to specify the resource.\n{}",
                                        resource_type_obj.name,
                                        resource_type_obj.description.as_deref().unwrap_or("No description"),
                                        if resources_count == 0 {
                                            "This resource does not have any available instances, you should create one from your windmill workspace."
                                        } else if resources_count > 1 {
                                            "This resource has multiple available instances, you should precisely select the one you want to use."
                                        } else {
                                            "There is 1 resource available."
                                        }
                                    ),
                                    None => "An object parameter.".to_string()
                                };
                                prop_map.insert(
                                    "type".to_string(),
                                    serde_json::Value::String("string".to_string()),
                                );
                                prop_map.insert(
                                    "description".to_string(),
                                    serde_json::Value::String(description),
                                );
                                if resources_count > 0 {
                                    let resources_description = resource_cache
                                        .iter()
                                        .map(|resource| {
                                            format!(
                                                "{}: $res:{}",
                                                resource
                                                    .description
                                                    .as_deref()
                                                    .unwrap_or("No title"),
                                                resource.path
                                            )
                                        })
                                        .collect::<Vec<String>>()
                                        .join("\n");

                                    prop_map.insert(
                                        "description".to_string(),
                                        serde_json::Value::String(format!(
                                            "{}\nHere are the available resources, in the format title:path. Title can be empty. Path should be used to specify the resource:\n{}",
                                            prop_map.get("description").unwrap_or(&serde_json::Value::String("No description".to_string())),
                                            resources_description
                                        )),
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                tracing::warn!(
                    "Schema property value is not a JSON object: {:?}",
                    prop_value
                );
            }
        }

        Ok(schema_obj)
    }

    /// Fetches the schema for a Hub script.
    ///
    /// This function takes a script path and a database connection, and attempts to fetch the schema for the script.
    /// It strips the path to remove any leading slashes, and then attempts to retrieve the full script using `get_full_hub_script_by_path`.
    /// If successful, it converts the schema string to a `Schema` object.
    /// If the schema cannot be converted, it logs a warning and returns `None`.
    ///
    /// # Parameters
    /// - `path`: The path of the script to fetch the schema for.
    /// - `db`: The database connection.
    ///
    /// # Returns
    /// - `Ok(Option<Schema>)`: The schema if found, otherwise `None`.
    /// - `Err(Error)`: If the request fails.
    async fn get_hub_script_schema(path: &str, db: &DB) -> Result<Option<Schema>, Error> {
        let strip_path = StripPath(path.to_string());
        let res = get_full_hub_script_by_path(strip_path, &HTTP_CLIENT, Some(db))
            .await
            .map_err(|e| {
                tracing::error!("Failed to get hub script: {}", e);
                Error::internal_error(format!("Failed to get hub script: {}", e), None)
            })?;
        match serde_json::from_str::<Schema>(res.schema.get()) {
            Ok(schema) => Ok(Some(schema)),
            Err(e) => {
                tracing::warn!("Failed to convert schema: {}", e);
                Ok(None)
            }
        }
    }

    /// Creates a `Tool` from a `ToolableItem`.
    ///
    /// This function takes an item that implements the `ToolableItem` trait and converts it into an RMCP `Tool`.
    /// It handles both workspace scripts/flows and Hub scripts differently, depending on the item type.
    ///
    /// # Parameters
    /// - `item`: The item to convert to a `Tool`.
    /// - `user_db`: The database connection.
    /// - `authed`: The authenticated user.
    /// - `workspace_id`: The workspace ID.
    /// - `resources_cache`: A mutable reference to the resources cache.
    /// - `resources_types`: A reference to the resource types.
    ///
    /// # Returns
    /// - `Ok(Tool)`: The created `Tool`.
    async fn create_tool_from_item<T: ToolableItem>(
        item: &T,
        user_db: &UserDB,
        authed: &ApiAuthed,
        workspace_id: &str,
        resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
        resources_types: &Vec<ResourceType>,
    ) -> Result<Tool, Error> {
        let is_hub = item.is_hub();
        let path = item.get_path_or_id();
        let item_type = item.item_type();
        let description = format!(
            "This is a {} named `{}` with the following description: `{}`.{}",
            item_type,
            item.get_summary(),
            item.get_description(),
            if is_hub {
                format!(
                    " It is a tool used for the following app: {}",
                    item.get_integration_type()
                        .unwrap_or("No integration type".to_string())
                )
            } else {
                "".to_string()
            }
        );
        let schema_obj = Runner::transform_schema_for_resources(
            &item.get_schema(),
            user_db,
            authed,
            &workspace_id,
            resources_cache,
            &resources_types,
        )
        .await?;
        let input_schema_map = match serde_json::to_value(schema_obj) {
            Ok(Value::Object(map)) => map,
            Ok(_) => {
                tracing::warn!("Schema object for tool '{}' did not serialize to a JSON object, using empty schema.", path);
                serde_json::Map::new()
            }
            Err(e) => {
                tracing::error!(
                    "Failed to serialize schema object for tool '{}': {}. Using empty schema.",
                    path,
                    e
                );
                serde_json::Map::new()
            }
        };
        Ok(Tool {
            name: Cow::Owned(path),
            description: Some(Cow::Owned(description)),
            input_schema: Arc::new(input_schema_map),
            annotations: None,
        })
    }
}

async fn call_endpoint_tool(
    tool: &crate::mcp_tools::EndpointTool,
    args: serde_json::Value,
    workspace_id: &str,
    api_authed: &ApiAuthed,
) -> Result<serde_json::Value, Error> {
    let method = &tool.method;
    let mut path_template = tool.path.to_string();
    
    // Always substitute {workspace} with the current workspace_id first
    path_template = path_template.replace("{workspace}", workspace_id);
    
    // Substitute other path parameters from args
    if let serde_json::Value::Object(args_map) = &args {
        if let Some(path_schema) = &tool.path_params_schema {
            if let Some(path_props) = path_schema.get("properties").and_then(|p| p.as_object()) {
                for (param_name, _) in path_props {
                    let placeholder = format!("{{{}}}", param_name);
                    if path_template.contains(&placeholder) {
                        if let Some(param_value) = args_map.get(param_name) {
                            if let Some(str_val) = param_value.as_str() {
                                path_template = path_template.replace(&placeholder, str_val);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Build query parameters
    let mut query_params = Vec::new();
    if let serde_json::Value::Object(args_map) = &args {
        if let Some(query_schema) = &tool.query_params_schema {
            if let Some(query_props) = query_schema.get("properties").and_then(|p| p.as_object()) {
                for (param_name, _) in query_props {
                    if let Some(value) = args_map.get(param_name) {
                        if let Some(str_val) = value.as_str() {
                            query_params.push(format!("{}={}", 
                                urlencoding::encode(param_name), 
                                urlencoding::encode(str_val)
                            ));
                        } else if let Some(num_val) = value.as_number() {
                            query_params.push(format!("{}={}", param_name, num_val));
                        } else if let Some(bool_val) = value.as_bool() {
                            query_params.push(format!("{}={}", param_name, bool_val));
                        }
                    }
                }
            }
        }
    }
    
    let query_string = if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    };

    // Prepare body for non-GET requests
    let body_json = if method != &http::Method::GET {
        if let Some(body_schema) = &tool.body_schema {
            if let serde_json::Value::Object(args_map) = &args {
                if let Some(body_props) = body_schema.get("properties").and_then(|p| p.as_object()) {
                    let mut body_map = serde_json::Map::new();
                    for (param_name, _) in body_props {
                        if let Some(value) = args_map.get(param_name) {
                            body_map.insert(param_name.clone(), value.clone());
                        }
                    }
                    if !body_map.is_empty() {
                        Some(serde_json::Value::Object(body_map))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let full_url = format!("{}/api{}{}", BASE_URL.read().await, path_template, query_string);

    // Create the HTTP request
    let client = &crate::HTTP_CLIENT;
    let mut request_builder = match method {
        &http::Method::GET => client.get(&full_url),
        &http::Method::POST => client.post(&full_url),
        &http::Method::PUT => client.put(&full_url),
        &http::Method::DELETE => client.delete(&full_url),
        &http::Method::PATCH => client.patch(&full_url),
        _ => return Err(Error::invalid_params(
            format!("Unsupported HTTP method: {}", method),
            Some(tool.name.clone().into())
        )),
    };

    // Add authorization header
    let authed = Authed::from(api_authed.clone());
    let token = create_jwt_token(authed, workspace_id, None, 3600, None).await
        .map_err(|e| Error::internal_error(e.to_string(), None))?;
    request_builder = request_builder.header("Authorization", format!("Bearer {}", token));

    // Add body for non-GET requests
    if let Some(body) = body_json {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .json(&body);
    }

    // Execute the request
    let response = request_builder.send().await.map_err(|e| {
        Error::internal_error(format!("Failed to execute request: {}", e), None)
    })?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        Error::internal_error(format!("Failed to read response text: {}", e), None)
    })?;

    if status.is_success() {
        // Try to parse as JSON, fallback to string if not valid JSON
        match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json_value) => Ok(json_value),
            Err(_) => Ok(serde_json::Value::String(response_text)),
        }
    } else {
        Err(Error::internal_error(
            format!("HTTP {} {}: {}", status.as_u16(), status.canonical_reason().unwrap_or(""), response_text),
            None
        ))
    }
}

impl ServerHandler for Runner {
    /// Handles the `CallTool` request from the MCP client.
    ///
    /// This involves:
    /// 1. Parsing arguments and extracting context (DB, Auth).
    /// 2. Reversing the tool name (`request.name`) to get the original path and type using `reverse_transform`.
    /// 3. Handling Hub scripts: If identified as a Hub script, searches the Hub for the actual script ID.
    /// 4. Fetching the schema for the item (needed for argument transformation).
    /// 5. Transforming incoming arguments:
    ///    - Reversing key transformations (e.g., `user_input` back to `user input`).
    ///    - Parsing stringified JSON objects back into JSON values based on schema type.
    /// 6. Executing the corresponding script or flow using internal Windmill runners.
    /// 7. Formatting the execution result into an RMCP `CallToolResult`.
    ///
    /// # Parameters
    /// - `request`: The `CallToolRequestParam` containing the tool name and arguments.
    /// - `context`: The `RequestContext` providing access to workspace ID, DB connections, auth info.
    ///
    /// # Returns
    /// - `Ok(CallToolResult)`: On successful execution, containing the output.
    /// - `Err(Error)`: If any step fails (parsing, DB access, execution, reversing transform, hub search).
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, Error> {

        let http_parts = context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| {
                tracing::error!("http::request::Parts not found");
                Error::internal_error("http::request::Parts not found", None)
            })?;

        let authed = http_parts.extensions.get::<ApiAuthed>().ok_or_else(|| {
            tracing::error!("ApiAuthed Axum extension not found");
            Error::internal_error("ApiAuthed Axum extension not found", None)
        })?;

        Runner::check_scopes(authed)?;

        let db = http_parts.extensions.get::<DB>().ok_or_else(|| {
            tracing::error!("DB Axum extension not found");
            Error::internal_error("DB Axum extension not found", None)
        })?;

        let user_db = http_parts.extensions.get::<UserDB>().ok_or_else(|| {
            tracing::error!("UserDB Axum extension not found");
            Error::internal_error("UserDB Axum extension not found", None)
        })?;
        
        let args = request.arguments.map(Value::Object).ok_or_else(|| {
            Error::invalid_params("Missing arguments for tool", Some(request.name.clone().into()))
        })?;

        let workspace_id = http_parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or_else(|| {
                tracing::error!("WorkspaceId not found");
                Error::internal_error("WorkspaceId not found", None)
            })
            .map(|w_id| w_id.0.clone())?;

        // Check if this is a generated endpoint tool
        let endpoint_tools = all_tools();
        for endpoint_tool in endpoint_tools {
            if endpoint_tool.name.as_ref() == request.name {
                // This is an endpoint tool, forward to the actual HTTP endpoint
                let result = call_endpoint_tool(&endpoint_tool, args.clone(), &workspace_id, &authed).await?;
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
                )]));
            }
        }

        // Continue with script/flow logic
        let (tool_type, path, is_hub) =
            Runner::reverse_transform(&request.name).unwrap_or_default();

        let item_schema = if is_hub {
            Runner::get_hub_script_schema(&format!("hub/{}", path), db).await?
        } else {
            Runner::get_item_schema(&path, user_db, authed, &workspace_id, &tool_type).await?
        };

        let schema_obj = if let Some(ref s) = item_schema {
            match serde_json::from_str::<SchemaType>(s.0.get()) {
                Ok(val) => Some(val),
                Err(e) => {
                    tracing::warn!("Failed to parse schema: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let push_args = if let Value::Object(map) = args.clone() {
            let mut args_hash = HashMap::new();
            for (k, v) in map {
                // need to transform back the key without invalid characters to the original key
                let original_key = Runner::reverse_transform_key(&k, &schema_obj);
                args_hash.insert(original_key, to_raw_value(&v));
            }
            windmill_queue::PushArgsOwned { extra: None, args: args_hash }
        } else {
            windmill_queue::PushArgsOwned::default()
        };
        let script_or_flow_path = if is_hub {
            StripPath(format!("hub/{}", path))
        } else {
            StripPath(path)
        };
        let run_query = RunJobQuery::default();

        let result = if tool_type == "script" {
            run_wait_result_script_by_path_internal(
                db.clone(),
                run_query,
                script_or_flow_path,
                authed.clone(),
                user_db.clone(),
                workspace_id.clone(),
                push_args,
            )
            .await
        } else {
            run_wait_result_flow_by_path_internal(
                db.clone(),
                run_query,
                script_or_flow_path,
                authed.clone(),
                user_db.clone(),
                push_args,
                workspace_id.clone(),
            )
            .await
        };

        match result {
            Ok(response) => {
                let body_bytes = to_bytes(response.into_body(), usize::MAX)
                    .await
                    .map_err(|e| {
                        Error::internal_error(format!("Failed to read response body: {}", e), None)
                    })?;
                let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
                    Error::internal_error(format!("Failed to decode response body: {}", e), None)
                })?;
                Ok(CallToolResult::success(vec![Content::text(body_str)]))
            }
            Err(e) => Err(Error::internal_error(
                format!("Failed to run script: {}", e),
                None,
            )),
        }
    }

    /// Fetches available tools (scripts, flows, hub scripts) based on the user's scope.
    ///
    /// - Determines scope (all, favorites, hub-specific) from auth token.
    /// - Fetches relevant items (workspace scripts/flows, hub scripts) concurrently.
    /// - Fetches resource type information needed for schema enrichment.
    /// - Transforms each item into an RMCP `Tool` definition, including schema adjustments
    ///   (like resource description enrichment and object->string conversion).
    ///
    /// # Parameters
    /// - `_request`: Optional pagination parameters (currently ignored).
    /// - `_context`: The `RequestContext` providing workspace ID, DB, auth.
    ///
    /// # Returns
    /// - `Ok(ListToolsResult)`: A list of `Tool` definitions. Pagination is not yet implemented.
    /// - `Err(Error)`: If fetching data from DB or Hub fails.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        mut _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, Error> {
        let http_parts = _context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| {
                tracing::error!("http::request::Parts not found");
                Error::internal_error("http::request::Parts not found", None)
            })?;

        let authed = http_parts.extensions.get::<ApiAuthed>().ok_or_else(|| {
            tracing::error!("ApiAuthed Axum extension not found");
            Error::internal_error("ApiAuthed Axum extension not found", None)
        })?;

        Runner::check_scopes(authed)?;

        let db = http_parts.extensions.get::<DB>().ok_or_else(|| {
            tracing::error!("DB Axum extension not found");
            Error::internal_error("DB Axum extension not found", None)
        })?;

        let user_db = http_parts.extensions.get::<UserDB>().ok_or_else(|| {
            tracing::error!("UserDB Axum extension not found");
            Error::internal_error("UserDB Axum extension not found", None)
        })?;

        let workspace_id = http_parts
            .extensions
            .get::<WorkspaceId>()
            .ok_or_else(|| {
                tracing::error!("WorkspaceId not found");
                Error::internal_error("WorkspaceId not found", None)
            })
            .map(|w_id| w_id.0.clone())?;

        let scopes = authed.scopes.as_ref();
        let owned_scope = scopes.and_then(|scopes| {
            scopes
                .iter()
                .find(|scope| scope.starts_with("mcp:") && !scope.contains("hub"))
        });
        let hub_scope = scopes.and_then(|scopes| scopes.iter().find(|scope| scope.starts_with("mcp:hub")));
        let scope_type = owned_scope.map_or("all", |scope| {
            let parts = scope.split(":").collect::<Vec<&str>>();
            parts[1]
        });
        let scope_integrations = hub_scope.and_then(|scope| {
            let parts = scope.split(":").collect::<Vec<&str>>();
            if parts.len() == 3 {
                Some(parts[2])
            } else {
                None
            }
        });

        let scripts_fn = Runner::inner_get_items::<ScriptInfo>(
            user_db,
            authed,
            &workspace_id,
            scope_type,
            "script",
        );
        let flows_fn =
            Runner::inner_get_items::<FlowInfo>(user_db, authed, &workspace_id, scope_type, "flow");
        let resources_types_fn = Runner::inner_get_resources_types(user_db, authed, &workspace_id);
        let hub_scripts_fn = Runner::inner_get_scripts_from_hub(db, scope_integrations.as_deref());
        let (scripts, flows, resources_types, hub_scripts) = if scope_integrations.is_some() {
            let (scripts, flows, resources_types, hub_scripts) =
                try_join!(scripts_fn, flows_fn, resources_types_fn, hub_scripts_fn)?;
            (scripts, flows, resources_types, hub_scripts)
        } else {
            let (scripts, flows, resources_types) =
                try_join!(scripts_fn, flows_fn, resources_types_fn)?;
            (scripts, flows, resources_types, vec![])
        };

        let mut resources_cache: HashMap<String, Vec<ResourceInfo>> = HashMap::new();
        let mut tools: Vec<Tool> = Vec::new();

        for script in scripts {
            tools.push(
                Runner::create_tool_from_item(
                    &script,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        for flow in flows {
            tools.push(
                Runner::create_tool_from_item(
                    &flow,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        for hub_script in hub_scripts {
            tools.push(
                Runner::create_tool_from_item(
                    &hub_script,
                    user_db,
                    authed,
                    &workspace_id,
                    &mut resources_cache,
                    &resources_types,
                )
                .await?,
            );
        }

        // Add endpoint tools from the generated MCP tools
        let endpoint_tools = all_tools();
        let mcp_tools_converted = endpoint_tools_to_mcp_tools(endpoint_tools);
        tools.extend(mcp_tools_converted);

        Ok(ListToolsResult { tools, next_cursor: None })
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a list of scripts and flows the user can run on Windmill. Each flow and script is a tool callable with their respective arguments.".to_string()),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, Error> {
        Ok(self.get_info())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, Error> {
        Ok(ListResourcesResult { resources: vec![], next_cursor: None })
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, Error> {
        Ok(ListPromptsResult::default())
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, Error> {
        Ok(ListResourceTemplatesResult::default())
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceId(pub String);

pub async fn extract_and_store_workspace_id(
    Path(params): Path<String>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let workspace_id = params;
    request.extensions_mut().insert(WorkspaceId(workspace_id));
    next.run(request).await
}

pub async fn setup_mcp_server() -> anyhow::Result<(Router, Arc<LocalSessionManager>)> {
    let session_manager = Arc::new(LocalSessionManager::default());
    let service_config = Default::default();
    let service = StreamableHttpService::new(
        || Ok(Runner::new()),
        session_manager.clone(),
        service_config,
    );

    let router = axum::Router::new().nest_service("/", service);
    Ok((router, session_manager))
}

pub async fn shutdown_mcp_server(session_manager: Arc<LocalSessionManager>) {
    let session_ids_to_close = {
        let sessions_map = session_manager.sessions.read().await;
        sessions_map.keys().cloned().collect::<Vec<_>>()
    };

    if !session_ids_to_close.is_empty() {
        tracing::info!(
            "Closing {} active MCP session(s)...",
            session_ids_to_close.len()
        );
        let close_futures = session_ids_to_close
            .iter()
            .map(|session_id| {
                let manager_clone = session_manager.clone();
                async move {
                    if let Err(_) = manager_clone.close_session(session_id).await {
                        tracing::warn!("Error closing MCP session");
                    }
                }
            })
            .collect::<Vec<_>>();
        futures::future::join_all(close_futures).await;
    }
}
