use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use std::fmt::Debug;
use windmill_common::{
    error::{Error, Result},
    DB,
};

use crate::db::ApiAuthed;

pub mod handler;
pub mod nextcloud;
pub mod sync;

/// Service name enum for native triggers
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "native_trigger_service", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceName {
    Nextcloud,
}

impl ServiceName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "nextcloud" => Ok(ServiceName::Nextcloud),
            _ => Err(Error::BadRequest(format!("Unknown service name: {}", s))),
        }
    }
}

/// Native trigger record in Windmill database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NativeTrigger {
    pub service_name: ServiceName,
    pub external_id: String,
    pub path: String,
    pub workspace_id: String,
    pub resource_path: String,
    pub summary: String,
    pub metadata: Option<serde_json::Value>,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
    pub extra_perms: Option<serde_json::Value>,
}

/// Request data for creating/updating a native trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTriggerData<P: Debug> {
    pub path: String,
    pub summary: Option<String>,
    #[serde(flatten)]
    pub payload: P,
}

/// Metadata extracted from external service payload
#[derive(Debug, Clone)]
pub struct TriggerMetadata {
    pub external_id: String,
    pub resource_path: String,
    pub summary: String,
    pub metadata: Option<serde_json::Value>,
}

/// Trait for external service implementations
#[async_trait]
pub trait External: Send + Sync + 'static {
    type Payload: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TriggerData: Debug + Serialize + Send + Sync;
    type Resource: Debug + DeserializeOwned + Send + Sync;

    const SERVICE_NAME: ServiceName;
    const DISPLAY_NAME: &'static str;
    const RESOURCE_TYPE: &'static str;

    async fn create(
        &self,
        resource: &Self::Resource,
        path: &str,
        payload: &Self::Payload,
    ) -> Result<TriggerMetadata>;

    async fn update(
        &self,
        resource: &Self::Resource,
        external_id: &str,
        path: &str,
        payload: &Self::Payload,
    ) -> Result<TriggerMetadata>;

    async fn get(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<Self::TriggerData>;

    async fn delete(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<()>;

    async fn exists(
        &self,
        resource: &Self::Resource,
        external_id: &str,
    ) -> Result<bool>;

    fn extract_metadata_from_payload(
        &self,
        payload: &Self::Payload,
        external_id: Option<&str>,
    ) -> Result<TriggerMetadata>;

    async fn list_all(
        &self,
        resource: &Self::Resource,
    ) -> Result<Vec<Self::TriggerData>>;

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }
}

pub async fn store_native_trigger(
    tx: &mut PgConnection,
    authed: &ApiAuthed,
    workspace_id: &str,
    path: &str,
    service_name: ServiceName,
    metadata: TriggerMetadata,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO native_triggers (
            service_name,
            external_id,
            path,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at,
            extra_perms
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, now(), $10
        )
        "#,
        service_name as ServiceName,
        metadata.external_id,
        path,
        workspace_id,
        metadata.resource_path,
        metadata.summary,
        metadata.metadata,
        authed.username,
        authed.email,
        None::<serde_json::Value>
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

pub async fn update_native_trigger(
    tx: &mut PgConnection,
    authed: &ApiAuthed,
    workspace_id: &str,
    old_path: &str,
    new_path: &str,
    service_name: ServiceName,
    metadata: TriggerMetadata,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE native_triggers
        SET
            path = $1,
            external_id = $2,
            resource_path = $3,
            summary = $4,
            metadata = $5,
            edited_by = $6,
            email = $7,
            edited_at = now()
        WHERE
            workspace_id = $8
            AND path = $9
            AND service_name = $10
        "#,
        new_path,
        metadata.external_id,
        metadata.resource_path,
        metadata.summary,
        metadata.metadata,
        authed.username,
        authed.email,
        workspace_id,
        old_path,
        service_name as ServiceName
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

pub async fn get_native_trigger(
    db: &DB,
    workspace_id: &str,
    path: &str,
    service_name: ServiceName,
) -> Result<NativeTrigger> {
    sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            service_name,
            external_id,
            path,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at,
            extra_perms
        FROM
            native_triggers
        WHERE
            workspace_id = $1
            AND path = $2
            AND service_name = $3
        "#,
        workspace_id,
        path,
        service_name as ServiceName
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Native trigger not found at path: {}", path)))
}

pub async fn delete_native_trigger(
    tx: &mut PgConnection,
    workspace_id: &str,
    path: &str,
    service_name: ServiceName,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE FROM native_triggers
        WHERE
            workspace_id = $1
            AND path = $2
            AND service_name = $3
        "#,
        workspace_id,
        path,
        service_name as ServiceName
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}

pub async fn exists_native_trigger(
    db: &DB,
    workspace_id: &str,
    path: &str,
    service_name: ServiceName,
) -> Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM native_triggers
            WHERE workspace_id = $1 AND path = $2 AND service_name = $3
        )
        "#,
        workspace_id,
        path,
        service_name as ServiceName
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    Ok(exists)
}

pub async fn get_native_trigger_by_external_id(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
) -> Result<Option<NativeTrigger>> {
    let trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            service_name,
            external_id,
            path,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at,
            extra_perms
        FROM
            native_triggers
        WHERE
            workspace_id = $1
            AND service_name = $2
            AND external_id = $3
        "#,
        workspace_id,
        service_name as ServiceName,
        external_id
    )
    .fetch_optional(db)
    .await?;

    Ok(trigger)
}

pub async fn list_native_triggers(
    db: &DB,
    workspace_id: &str,
    service_name: ServiceName,
    page: Option<usize>,
    per_page: Option<usize>,
) -> Result<Vec<NativeTrigger>> {
    let offset = (page.unwrap_or(0) * per_page.unwrap_or(100)) as i64;
    let limit = per_page.unwrap_or(100) as i64;

    let triggers = sqlx::query_as!(
        NativeTrigger,
        r#"
            SELECT
                service_name,
                external_id,
                path,
                workspace_id,
                resource_path,
                summary,
                metadata,
                edited_by,
                email,
                edited_at,
                extra_perms
            FROM
                native_triggers
            WHERE
                workspace_id = $1
                AND service_name = $2
            ORDER BY edited_at DESC
            LIMIT $3 OFFSET $4
            "#,
        workspace_id,
        service_name as ServiceName,
        limit,
        offset
    )
    .fetch_all(db)
    .await?;

    Ok(triggers)
}

#[derive(Debug, Clone)]
pub struct ClientWithAuth<T> {
    pub client: reqwest::Client,
    pub auth_data: T,
}

impl<T> ClientWithAuth<T> {
    pub fn new(auth_data: T) -> Self {
        Self { client: reqwest::Client::new(), auth_data }
    }
}

pub async fn get_resource_for_native_trigger<T>(
    db: &DB,
    workspace_id: &str,
    resource_type: &str,
) -> Result<ClientWithAuth<T>>
where
    T: serde::de::DeserializeOwned,
{
    let resource = sqlx::query!(
        r#"
        SELECT value, path
        FROM resource
        WHERE workspace_id = $1
          AND resource_type = $2
        LIMIT 1
        "#,
        workspace_id,
        resource_type
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        Error::BadConfig(format!(
            "No '{}' resource found in workspace '{}'. Please create a resource with type '{}' containing the required credentials.",
            resource_type, workspace_id, resource_type
        ))
    })?;

    let auth_data: T = serde_json::from_value(resource.value.clone()).map_err(|e| {
        Error::BadConfig(format!(
            "Invalid '{}' resource at path '{}': {}. Please check the resource schema.",
            resource_type, resource.path, e
        ))
    })?;

    Ok(ClientWithAuth::new(auth_data))
}
