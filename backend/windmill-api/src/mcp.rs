use std::borrow::Cow;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::Router;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
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
use tokio_util::sync::CancellationToken;
use windmill_common::db::UserDB;
use windmill_common::worker::to_raw_value;
use windmill_common::{DB, HUB_BASE_URL};

use windmill_common::scripts::{get_full_hub_script_by_path, Schema};

use crate::db::ApiAuthed;
use crate::jobs::{
    run_wait_result_flow_by_path_internal, run_wait_result_script_by_path_internal, RunJobQuery,
};
use crate::HTTP_CLIENT;
use windmill_common::utils::{query_elems_from_hub, StripPath};

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
        sqlb.fields(&["o.path", "o.summary", "o.description", "o.schema"]);
        if scope_type == "favorites" {
            sqlb.join("favorite")
                .on("favorite.favorite_kind = ? AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?".bind(&item_type)
                    .bind(&authed.username));
        }
        sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
            .and_where("o.archived = false")
            .and_where("o.draft_only IS NOT TRUE")
            .order_by(
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

    /// Transforms a value if it's an object.
    ///
    /// This function takes a key and a value, and a schema object.
    /// If the value is a string that starts with "$res:", it returns the value as is.
    /// Otherwise, it checks if the key is defined in the schema and if it's an object type.
    /// If it is, it transforms the value to a string. This is because some clients do not support object types.
    /// # Parameters
    /// - `key`: The key of the value to transform.
    /// - `value`: The value to transform.
    /// - `schema_obj`: The schema object.
    ///
    /// # Returns
    /// - `Value`: The transformed value.
    fn transform_value_if_object(
        key: &str,
        value: &Value,
        schema_obj: &Option<SchemaType>,
    ) -> Value {
        if value.is_string() && value.as_str().unwrap().starts_with("$res:") {
            return value.clone();
        }

        let schema_obj = match schema_obj {
            Some(s) => s,
            None => return value.clone(),
        };

        // Check if property is defined in schema and is an object type
        let is_obj_type = match schema_obj.properties.get(key) {
            Some(property) => {
                let prop_type = property.get("type").and_then(|t| t.as_str());
                prop_type == Some("object")
            }
            None => false,
        };

        // If it's an object type and we received a string, try to parse it
        if is_obj_type && value.is_string() {
            if let Some(str_val) = value.as_str() {
                if let Ok(obj_val) = serde_json::from_str::<serde_json::Value>(str_val) {
                    return obj_val;
                }
            }
        }

        value.clone()
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
                // transform object properties to string because some client does not support object, might change in the future
                if let Some(type_value) = prop_map.get("type") {
                    if let serde_json::Value::String(type_str) = type_value {
                        if type_str == "object" {
                            prop_map.insert(
                                "type".to_string(),
                                serde_json::Value::String("string".to_string()),
                            );
                        }
                    }
                }
                // if property is a resource, fetch the resource type infos, and add each available resource to the description
                if let Some(format_value) = prop_map.get("format") {
                    if let serde_json::Value::String(format_str) = format_value {
                        if format_str.starts_with("resource-") {
                            let resource_type_key =
                                format_str.split("-").last().unwrap_or_default().to_string();
                            let resource_type = resources_types
                                .iter()
                                .find(|rt| rt.name == resource_type_key);
                            let resource_type_obj = resource_type.cloned().unwrap_or_else(|| {
                                tracing::info!("Resource type not found: {}", resource_type_key);
                                ResourceType { name: resource_type_key.clone(), description: None }
                            });

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
                                let description = format!(
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
                                );
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
        let parse_args = |args_opt: Option<JsonObject>| -> Result<Value, Error> {
            args_opt.map(Value::Object).ok_or_else(|| {
                Error::invalid_params(
                    "Missing arguments for tool",
                    Some(request.name.clone().into()),
                )
            })
        };

        let authed = context
            .req_extensions
            .get::<ApiAuthed>()
            .ok_or_else(|| Error::internal_error("ApiAuthed not found", None))?;
        let db = context
            .req_extensions
            .get::<DB>()
            .ok_or_else(|| Error::internal_error("DB not found", None))?;
        let user_db = context
            .req_extensions
            .get::<UserDB>()
            .ok_or_else(|| Error::internal_error("UserDB not found", None))?;
        let args = parse_args(request.arguments)?;

        let (tool_type, path, is_hub) =
            Runner::reverse_transform(&request.name).unwrap_or_default();

        let item_schema = if is_hub {
            Runner::get_hub_script_schema(&format!("hub/{}", path), db).await?
        } else {
            Runner::get_item_schema(&path, user_db, authed, &context.workspace_id, &tool_type)
                .await?
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

                // object properties are transformed to string because some client does not support object, might change in the future
                let transformed_v = Runner::transform_value_if_object(&k, &v, &schema_obj);
                args_hash.insert(original_key, to_raw_value(&transformed_v));
            }
            windmill_queue::PushArgsOwned { extra: None, args: args_hash }
        } else {
            windmill_queue::PushArgsOwned::default()
        };

        let w_id = context.workspace_id.clone();
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
                w_id.clone(),
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
                w_id.clone(),
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
        let workspace_id = _context.workspace_id.clone();
        let db = _context
            .req_extensions
            .get::<DB>()
            .ok_or_else(|| Error::internal_error("DB not found", None))?;
        let user_db = _context
            .req_extensions
            .get::<UserDB>()
            .ok_or_else(|| Error::internal_error("UserDB not found", None))?;
        let authed = _context
            .req_extensions
            .get::<ApiAuthed>()
            .ok_or_else(|| Error::internal_error("ApiAuthed not found", None))?;
        let owned_scope = authed.scopes.as_ref().and_then(|scopes| {
            scopes
                .iter()
                .find(|scope| scope.starts_with("mcp:") && !scope.contains("hub"))
        });
        let hub_scope = authed
            .scopes
            .as_ref()
            .and_then(|scopes| scopes.iter().find(|scope| scope.starts_with("mcp:hub")));
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

pub fn setup_mcp_server(addr: SocketAddr, path: &str) -> anyhow::Result<(SseServer, Router)> {
    let config = SseServerConfig {
        bind: addr,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        full_message_path: path.to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: None,
    };

    Ok(SseServer::new(config))
}
