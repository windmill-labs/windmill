use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Secret;
use kube::api::Api;
use kube::Client;
use windmill_common::instance_config::{GlobalSettings, SecretKeyRef, StringOrSecretRef};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("failed to fetch Kubernetes Secret '{name}': {source}")]
    FetchSecret { name: String, source: kube::Error },
    #[error("key '{key}' not found in Secret '{secret}'")]
    KeyNotFound { secret: String, key: String },
    #[error("value for key '{key}' in Secret '{secret}' is not valid UTF-8")]
    InvalidUtf8 { secret: String, key: String },
}

/// Resolve all `StringOrSecretRef` fields in `GlobalSettings` by reading
/// referenced Kubernetes Secrets. Returns a new `GlobalSettings` with every
/// `SecretRef` replaced by its `Literal` value.
pub async fn resolve_secret_refs(
    client: &Client,
    namespace: &str,
    settings: &GlobalSettings,
) -> Result<GlobalSettings, ResolveError> {
    let mut settings = settings.clone();
    let mut cache: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    resolve_option(client, namespace, &mut cache, &mut settings.license_key).await?;
    resolve_option(client, namespace, &mut cache, &mut settings.hub_api_secret).await?;
    resolve_option(client, namespace, &mut cache, &mut settings.scim_token).await?;

    if let Some(smtp) = &mut settings.smtp_settings {
        resolve_option(client, namespace, &mut cache, &mut smtp.smtp_password).await?;
    }

    if let Some(oauths) = &mut settings.oauths {
        for oauth in oauths.values_mut() {
            resolve_field(client, namespace, &mut cache, &mut oauth.secret).await?;
        }
    }

    if let Some(pg) = &mut settings.custom_instance_pg_databases {
        resolve_option(client, namespace, &mut cache, &mut pg.user_pwd).await?;
    }

    Ok(settings)
}

async fn resolve_option(
    client: &Client,
    namespace: &str,
    cache: &mut BTreeMap<String, BTreeMap<String, String>>,
    field: &mut Option<StringOrSecretRef>,
) -> Result<(), ResolveError> {
    if let Some(val) = field {
        resolve_field(client, namespace, cache, val).await?;
    }
    Ok(())
}

async fn resolve_field(
    client: &Client,
    namespace: &str,
    cache: &mut BTreeMap<String, BTreeMap<String, String>>,
    field: &mut StringOrSecretRef,
) -> Result<(), ResolveError> {
    let secret_ref = match field.as_secret_ref() {
        Some(r) => r.clone(),
        None => return Ok(()),
    };

    let value = fetch_secret_value(client, namespace, cache, &secret_ref).await?;
    *field = StringOrSecretRef::Literal(value);
    Ok(())
}

async fn fetch_secret_value(
    client: &Client,
    namespace: &str,
    cache: &mut BTreeMap<String, BTreeMap<String, String>>,
    secret_ref: &SecretKeyRef,
) -> Result<String, ResolveError> {
    if let Some(data) = cache.get(&secret_ref.name) {
        return data
            .get(&secret_ref.key)
            .cloned()
            .ok_or_else(|| ResolveError::KeyNotFound {
                secret: secret_ref.name.clone(),
                key: secret_ref.key.clone(),
            });
    }

    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let secret = api
        .get(&secret_ref.name)
        .await
        .map_err(|e| ResolveError::FetchSecret { name: secret_ref.name.clone(), source: e })?;

    let data = secret.data.unwrap_or_default();
    let decoded: BTreeMap<String, String> = data
        .into_iter()
        .filter_map(|(k, v)| String::from_utf8(v.0).ok().map(|s| (k, s)))
        .collect();

    let value = decoded
        .get(&secret_ref.key)
        .cloned()
        .ok_or_else(|| ResolveError::KeyNotFound {
            secret: secret_ref.name.clone(),
            key: secret_ref.key.clone(),
        })?;

    cache.insert(secret_ref.name.clone(), decoded);
    Ok(value)
}
