use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use std::fmt::Debug;
use windmill_common::{
    error::{Error, Result},
    utils::RunnableKind,
    DB,
};
use windmill_queue::{PushArgs, PushArgsOwned};

use crate::db::ApiAuthed;

pub mod handler;
pub mod nextcloud;
pub mod sync;

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

impl std::fmt::Display for ServiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let service_name = match self {
            ServiceName::Nextcloud => "nextcloud",
        };

        write!(f, "{}", service_name)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NativeTrigger {
    pub id: i64,
    pub service_name: ServiceName,
    pub external_id: String,
    pub workspace_id: String,
    pub resource_path: String,
    pub runnable_path: String,
    pub runnable_kind: RunnableKind,
    pub summary: String,
    pub metadata: Option<serde_json::Value>,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeTriggerData<P> {
    pub external_id: String,
    pub runnable_path: String,
    pub runnable_kind: RunnableKind,
    pub resource_path: String,
    pub summary: Option<String>,
    #[serde(flatten)]
    pub payload: P,
}

impl<P> Into<TriggerMetadata> for NativeTriggerData<P> {
    fn into(self) -> TriggerMetadata {
        TriggerMetadata {
            external_id: self.external_id,
            resource_path: self.resource_path,
            summary: self.summary,
            runnable_kind: self.runnable_kind,
            runnable_path: self.runnable_path,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TriggerMetadata {
    pub external_id: String,
    pub runnable_path: String,
    pub runnable_kind: RunnableKind,
    pub resource_path: String,
    pub summary: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl TriggerMetadata {
    pub fn new<P>(external_id: String, native_trigger: NativeTriggerData<P>) -> TriggerMetadata {
        TriggerMetadata {
            external_id: external_id,
            resource_path: native_trigger.resource_path,
            summary: native_trigger.summary,
            runnable_kind: native_trigger.runnable_kind,
            runnable_path: native_trigger.runnable_path,
            metadata: None,
        }
    }
}

#[async_trait]
pub trait External: Send + Sync + 'static {
    type Payload: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TriggerData: Debug + Serialize + Send + Sync;
    type Resource: DeserializeOwned + Send + Sync;
    type CreateResponse: DeserializeOwned + Send + Sync;

    const SUPPORT_WEBHOOK: bool;
    const SERVICE_NAME: ServiceName;
    const DISPLAY_NAME: &'static str;
    const RESOURCE_TYPE: &'static str;

    async fn create(
        &self,
        w_id: &str,
        runnable_path: &str,
        resource: &Self::Resource,
        payload: &Self::Payload,
    ) -> Result<Self::CreateResponse>;

    async fn update(
        &self,
        w_id: &str,
        runnable_path: &str,
        resource: &Self::Resource,
        external_id: &str,
        payload: &Self::Payload,
    ) -> Result<()>;

    async fn get(&self, resource: &Self::Resource, external_id: &str) -> Result<Self::TriggerData>;

    async fn delete(&self, resource: &Self::Resource, external_id: &str) -> Result<()>;

    async fn exists(&self, resource: &Self::Resource, external_id: &str) -> Result<bool>;

    async fn list_all(&self, resource: &Self::Resource) -> Result<Vec<Self::TriggerData>>;

    async fn prepare_webhook() -> Option<Result<PushArgsOwned>> {
        None
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>);

    fn additional_routes(&self) -> axum::Router {
        axum::Router::new()
    }
}

pub async fn store_native_trigger(
    tx: &mut PgConnection,
    authed: &ApiAuthed,
    workspace_id: &str,
    service_name: ServiceName,
    metadata: TriggerMetadata,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO native_triggers (
            service_name,
            external_id,
            runnable_path,
            runnable_kind,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now()
        )
        "#,
        service_name as ServiceName,
        metadata.external_id,
        metadata.runnable_path,
        metadata.runnable_kind as RunnableKind,
        workspace_id,
        metadata.resource_path,
        metadata.summary,
        metadata.metadata,
        authed.username,
        authed.email,
    )
    .execute(&mut *tx)
    .await?;

    Ok(())
}

pub async fn update_native_trigger(
    tx: &mut PgConnection,
    authed: &ApiAuthed,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
    metadata: TriggerMetadata,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE native_triggers
        SET
            runnable_path = $1,
            runnable_kind = $2,
            external_id = $3,
            resource_path = $4,
            summary = $5,
            metadata = $6,
            edited_by = $7,
            email = $8,
            edited_at = now()
        WHERE
            workspace_id = $9
            AND id = $10
            AND service_name = $11
        "#,
        metadata.runnable_path,
        metadata.runnable_kind as RunnableKind,
        metadata.external_id,
        metadata.resource_path,
        metadata.summary,
        metadata.metadata,
        authed.username,
        authed.email,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .execute(tx)
    .await?;

    Ok(())
}

pub async fn get_native_trigger(
    tx: &mut PgConnection,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
) -> Result<NativeTrigger> {
    let native_trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            id,
            runnable_path,
            runnable_kind AS "runnable_kind!: RunnableKind",
            service_name AS "service_name!: ServiceName",
            external_id,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
        FROM
            native_triggers
        WHERE
            workspace_id = $1
            AND id = $2
            AND service_name = $3
        "#,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .fetch_optional(tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("Native trigger not found")));

    native_trigger
}

pub async fn delete_native_trigger(
    tx: &mut PgConnection,
    workspace_id: &str,
    id: i64,
    service_name: ServiceName,
) -> Result<bool> {
    let deleted = sqlx::query!(
        r#"
        DELETE 
            FROM native_triggers
        WHERE
            workspace_id = $1
            AND id = $2
            AND service_name = $3
        "#,
        workspace_id,
        id,
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
    id: i64,
    service_name: ServiceName,
) -> Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 
                1
            FROM 
                native_triggers
            WHERE 
                workspace_id = $1 AND 
                id = $2 AND 
                service_name = $3
        )
        "#,
        workspace_id,
        id,
        service_name as ServiceName
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false);

    Ok(exists)
}

pub async fn get_native_trigger_by_external_id(
    tx: &mut PgConnection,
    workspace_id: &str,
    service_name: ServiceName,
    external_id: &str,
) -> Result<Option<NativeTrigger>> {
    let trigger = sqlx::query_as!(
        NativeTrigger,
        r#"
        SELECT
            id,
            runnable_path,
            runnable_kind AS "runnable_kind!: RunnableKind",
            service_name AS "service_name!: ServiceName",
            external_id,
            workspace_id,
            resource_path,
            summary,
            metadata,
            edited_by,
            email,
            edited_at
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
    .fetch_optional(tx)
    .await?;

    Ok(trigger)
}

pub async fn list_native_triggers(
    tx: &mut PgConnection,
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
                id,
                runnable_path,
                runnable_kind AS "runnable_kind!: RunnableKind",
                service_name AS "service_name!: ServiceName",
                external_id,
                workspace_id,
                resource_path,
                summary,
                metadata,
                edited_by,
                email,
                edited_at
            FROM
                native_triggers
            WHERE
                workspace_id = $1 AND 
                service_name = $2
            ORDER BY 
                edited_at DESC
            LIMIT 
                $3 
            OFFSET 
                $4
            "#,
        workspace_id,
        service_name as ServiceName,
        limit,
        offset
    )
    .fetch_all(tx)
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
