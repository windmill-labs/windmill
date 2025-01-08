use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sqlx::prelude::FromRow;

use crate::DB;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Resource<T> {
    pub resource_type: String,
    pub value: T,
    pub extra_perms: Value,
}

impl<T> Resource<T> {
    pub fn new(resource_type: String, value: T, extra_perms: Value) -> Resource<T> {
        Resource { resource_type, value, extra_perms }
    }
}

pub async fn get_resource<'de, T>(
    db: &DB,
    path: &str,
    w_id: &str,
) -> Result<Resource<T>, sqlx::Error>
where
    T: Send + Sync + std::marker::Unpin + DeserializeOwned,
{
    let resource = sqlx::query_as!(Resource, "SELECT resource_type, value, extra_perms FROM resource WHERE path = $1 AND workspace_id = $2", path, w_id).fetch_one(db).await?;

    let val: T = serde_json::from_value(resource.value).unwrap();

    let resource = Resource::new(resource.resource_type, val, resource.extra_perms);

    Ok(resource)
}
