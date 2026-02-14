use std::{
    hash::DefaultHasher,
    sync::atomic::{AtomicI64, Ordering},
};

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{Authed, AuthedRef},
    error::{Error, Result},
    jwt,
    users::{SUPERADMIN_NOTIFICATION_EMAIL, SUPERADMIN_SECRET_EMAIL, SUPERADMIN_SYNC_EMAIL},
    utils::WarnAfterExt,
    DB,
};

#[derive(Debug)]
pub struct IdToken {
    token: String,
    expiration: DateTime<Utc>,
}

pub const TOKEN_PREFIX_LEN: usize = 10;

lazy_static::lazy_static! {
    // Cache for script hash permissions - (ApiAuthed hash, script_hash) -> permission result
    pub static ref HASH_PERMS_CACHE: PermsCache = PermsCache::new();
    pub static ref FLOW_PERMS_CACHE: PermsCache = PermsCache::new();
}

pub struct PermsCache(Cache<(u64, u64), ()>, AtomicI64);

use std::hash::Hash;
use std::hash::Hasher;

impl PermsCache {
    pub fn compute_hash(authed: &AuthedRef) -> u64 {
        let mut hasher = DefaultHasher::new();
        authed.username.hash(&mut hasher);
        authed.folders.hash(&mut hasher);
        authed.groups.hash(&mut hasher);
        authed.is_admin.hash(&mut hasher);
        hasher.finish()
    }
}

pub const PERMS_CACHE_EXPIRATION_SECONDS: i64 = 60 * 60;

impl PermsCache {
    pub fn new() -> Self {
        PermsCache(
            Cache::new(10000),
            AtomicI64::new(chrono::Utc::now().timestamp() as i64),
        )
    }

    pub fn check_perms_in_cache<'e, T: Into<u64>>(
        &self,
        authed: &'e AuthedRef<'e>,
        key: T,
    ) -> (bool, u64) {
        // Clear cache every hour
        if self.1.load(Ordering::Relaxed)
            < chrono::Utc::now().timestamp() - PERMS_CACHE_EXPIRATION_SECONDS
        {
            self.0.clear();
            self.1
                .store(chrono::Utc::now().timestamp() as i64, Ordering::Relaxed);
        }
        // Create hash of the ApiAuthed struct for caching
        let authed_hash = Self::compute_hash(authed);

        let key = key.into();
        tracing::debug!(
            "Checking cache for authed hash {authed_hash} and script hash {}",
            key
        );
        // Check cache first
        if let Some(_) = self.0.get(&(authed_hash, key)) {
            tracing::debug!("Cached result for authed hash {authed_hash}",);
            return (true, authed_hash);
        }

        return (false, authed_hash);
    }

    pub fn insert<'e, T: Into<u64>>(&self, authed_hash: u64, key: T) {
        let key = key.into();
        tracing::debug!("Inserting authed hash {authed_hash} and key {}", key);
        self.0.insert((authed_hash, key), ());
    }
}

pub fn has_expired(expiration_time: DateTime<Utc>, take: Option<Duration>) -> bool {
    let now = Utc::now();

    let expiration = match take {
        Some(duration) => expiration_time - duration,
        None => expiration_time,
    };

    now > expiration
}

impl From<IdToken> for String {
    fn from(value: IdToken) -> Self {
        value.token
    }
}

impl ToString for IdToken {
    fn to_string(&self) -> String {
        self.token.clone()
    }
}

impl IdToken {
    pub fn new(token: String, expiration: DateTime<Utc>) -> Self {
        Self { token, expiration }
    }

    pub fn token(&self) -> &str {
        &self.token
    }
    pub fn expiration(&self) -> &DateTime<Utc> {
        &self.expiration
    }
}

#[derive(Deserialize, Serialize)]
pub struct JWTAuthClaims {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<(String, bool, bool)>,
    pub label: Option<String>,
    pub workspace_id: Option<String>,
    pub workspace_ids: Option<Vec<String>>,
    pub exp: usize,
    pub job_id: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub audit_span: Option<String>,
}

impl JWTAuthClaims {
    pub fn allowed_in_workspace(&self, w_id: &str) -> bool {
        self.workspace_id
            .as_ref()
            .is_some_and(|token_w_id| w_id == token_w_id)
            || self
                .workspace_ids
                .as_ref()
                .is_some_and(|token_w_ids| token_w_ids.iter().any(|token_w_id| w_id == token_w_id))
    }

    pub fn compute_ext_jwt_hash(&self) -> i64 {
        let mut hasher = DefaultHasher::new();
        self.email.hash(&mut hasher);
        self.username.hash(&mut hasher);
        self.is_admin.hash(&mut hasher);
        self.is_operator.hash(&mut hasher);
        self.groups.hash(&mut hasher);
        self.folders.hash(&mut hasher);
        self.workspace_id.hash(&mut hasher);
        self.workspace_ids.hash(&mut hasher);
        self.label.hash(&mut hasher);
        self.scopes.hash(&mut hasher);
        hasher.finish() as i64
    }
}

#[derive(Deserialize, Debug)]
pub struct JobPerms {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    pub folders: Vec<serde_json::Value>,
}

impl From<JobPerms> for Authed {
    fn from(value: JobPerms) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value
                .folders
                .into_iter()
                .filter_map(|x| serde_json::from_value::<(String, bool, bool)>(x).ok())
                .collect(),
            scopes: None,
            token_prefix: None,
        }
    }
}

pub async fn is_super_admin_email(db: &DB, email: &str) -> Result<bool> {
    if email == SUPERADMIN_SECRET_EMAIL || email == SUPERADMIN_NOTIFICATION_EMAIL {
        return Ok(true);
    }

    let is_admin = sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_admin)
}

pub async fn is_devops_email(db: &DB, email: &str) -> Result<bool> {
    if is_super_admin_email(db, email).await? {
        return Ok(true);
    }

    let is_devops = sqlx::query_scalar!("SELECT devops FROM password WHERE email = $1", email)
        .fetch_optional(db)
        .await
        .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
        .unwrap_or(false);

    Ok(is_devops)
}

pub fn permissioned_as_to_username(permissioned_as: &str) -> String {
    if let Some((prefix, name)) = permissioned_as.split_once('/') {
        if prefix == "u" {
            name.to_string()
        } else {
            format!("group-{}", name)
        }
    } else {
        permissioned_as.to_string()
    }
}

pub fn fetch_authed_from_permissioned_as<'a, A>(
    permissioned_as: &'a str,
    email: &'a str,
    w_id: &'a str,
    db: A,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Authed>> + Send + 'a>>
where
    A: sqlx::Acquire<'a, Database = sqlx::Postgres> + Send + 'a,
{
    Box::pin(async move {
        let mut conn = db
            .acquire()
            .await
            .map_err(|e| Error::internal_err(format!("acquiring connection: {e:#}")))?;

        fetch_authed_from_permissioned_as_inner(permissioned_as, email, w_id, &mut *conn).await
    })
}

async fn fetch_authed_from_permissioned_as_inner(
    permissioned_as: &str,
    email: &str,
    w_id: &str,
    conn: &mut sqlx::PgConnection,
) -> Result<Authed> {
    let is_super_admin = permissioned_as == SUPERADMIN_SYNC_EMAIL
        || email == SUPERADMIN_SECRET_EMAIL
        || email == SUPERADMIN_NOTIFICATION_EMAIL
        || sqlx::query_scalar!("SELECT super_admin FROM password WHERE email = $1", email)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| Error::internal_err(format!("fetching super admin: {e:#}")))?
            .unwrap_or(false);

    if let Some((prefix, name)) = permissioned_as.split_once('/') {
        if prefix == "u" {
            let (is_admin, is_operator) = if is_super_admin {
                (true, false)
            } else {
                let r = sqlx::query!(
                    "SELECT is_admin, operator FROM usr where username = $1 AND \
                                                 workspace_id = $2 AND disabled = false",
                    name,
                    &w_id
                )
                .fetch_optional(&mut *conn)
                .await?;
                if let Some(r) = r {
                    (r.is_admin, r.operator)
                } else {
                    return Err(Error::internal_err(format!(
                        "user {name} not found in workspace {w_id}"
                    )));
                }
            };

            let groups = get_groups_for_user(w_id, &name, email, &mut *conn).await?;

            let folders = get_folders_for_user(w_id, &name, &groups, &mut *conn).await?;

            Ok(Authed {
                email: email.to_string(),
                username: name.to_string(),
                is_admin,
                is_operator,
                groups,
                folders,
                scopes: None,
                token_prefix: None,
            })
        } else {
            let groups = vec![name.to_string()];
            let folders = get_folders_for_user(&w_id, "", &groups, &mut *conn).await?;
            Ok(Authed {
                email: email.to_string(),
                username: format!("group-{name}"),
                is_admin: false,
                groups,
                is_operator: false,
                folders,
                scopes: None,
                token_prefix: None,
            })
        }
    } else {
        Ok(Authed {
            email: email.to_string(),
            username: permissioned_as.to_string(),
            is_admin: is_super_admin,
            is_operator: true,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        })
    }
}

pub async fn get_folders_for_user<'e, E: sqlx::PgExecutor<'e>>(
    w_id: &str,
    username: &str,
    groups: &[String],
    db: E,
) -> Result<Vec<(String, bool, bool)>> {
    let mut perms = groups
        .into_iter()
        .map(|x| format!("g/{}", x))
        .collect::<Vec<_>>();
    perms.insert(0, format!("u/{}", username));
    let folders = sqlx::query!(
        "SELECT name, (EXISTS (SELECT 1 FROM (SELECT key, value FROM jsonb_each_text(extra_perms) WHERE key = ANY($1)) t  WHERE value::boolean IS true)) as write, $1 && owners::text[] as owner  FROM folder
        WHERE extra_perms ?| $1  AND workspace_id = $2",
        &perms[..],
        w_id,
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|x| (x.name, x.write.unwrap_or(false), x.owner.unwrap_or(false)))
    .collect();

    Ok(folders)
}

pub async fn get_groups_for_user<'e, E: sqlx::PgExecutor<'e>>(
    w_id: &str,
    username: &str,
    email: &str,
    db: E,
) -> Result<Vec<String>> {
    let groups = sqlx::query_scalar!(
        "SELECT group_ FROM usr_to_group where usr = $1 AND workspace_id = $2 UNION ALL SELECT igroup FROM email_to_igroup WHERE email = $3",
        username,
        w_id,
        email
    )
    .fetch_all(db)
    .await?
    .into_iter().filter_map(|x| x)
    .collect();
    Ok(groups)
}

pub async fn get_job_perms<'a, E: sqlx::PgExecutor<'a>>(
    db: E,
    job_id: &Uuid,
    w_id: &str,
) -> sqlx::Result<Option<JobPerms>> {
    sqlx::query_as!(
        JobPerms,
        "SELECT email, username, is_admin, is_operator, groups, folders FROM job_perms WHERE job_id = $1 AND workspace_id = $2",
        job_id,
        w_id
    )
    .fetch_optional(db)
    .warn_after_seconds(3)
    .await
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn create_token_for_owner(
    db: &DB,
    w_id: &str,
    owner: &str,
    label: &str,
    expires_in: u64,
    email: &str,
    job_id: &Uuid,
    perms: Option<JobPerms>,
    audit_span: Option<String>,
) -> crate::error::Result<String> {
    let job_perms = if perms.is_some() {
        Ok(perms)
    } else {
        get_job_perms(db, job_id, w_id).await
    };
    let job_authed = match job_perms {
        Ok(Some(jp)) => jp.into(),
        _ => {
            tracing::warn!("Could not get permissions for job {job_id} from job_perms table, getting permissions directly...");
            fetch_authed_from_permissioned_as(owner, email, w_id, db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "Could not get permissions directly for job {job_id}: {e:#}"
                    ))
                })?
        }
    };

    create_jwt_token(
        job_authed,
        w_id,
        expires_in,
        Some(*job_id),
        Some(label.to_string()),
        audit_span,
        None,
    )
    .await
}

pub async fn create_jwt_token(
    authed: Authed,
    workspace_id: &str,
    expires_in_seconds: u64,
    job_id: Option<Uuid>,
    label: Option<String>,
    audit_span: Option<String>,
    scopes: Option<Vec<String>>,
) -> crate::error::Result<String> {
    let payload = JWTAuthClaims {
        email: authed.email.clone(),
        username: authed.username.clone(),
        is_admin: authed.is_admin,
        is_operator: authed.is_operator,
        groups: authed.groups.clone(),
        folders: authed.folders.clone(),
        label,
        workspace_id: Some(workspace_id.to_string()),
        workspace_ids: None,
        exp: (chrono::Utc::now() + chrono::Duration::seconds(expires_in_seconds as i64)).timestamp()
            as usize,
        job_id: job_id.map(|id| id.to_string()),
        scopes,
        audit_span,
    };

    let token = jwt::encode_with_internal_secret(&payload)
        .await
        .with_context(|| match job_id {
            Some(job_id) => format!("Could not encode JWT token for job {job_id}"),
            None => "Could not encode JWT token".to_string(),
        })?;

    Ok(format!("jwt_{}", token))
}

#[cfg(feature = "aws_auth")]
pub mod aws {

    use super::*;
    use crate::utils::empty_as_none;
    use aws_config::{BehaviorVersion, Region};
    use aws_sdk_sts::{
        config::Credentials as AwsCredentials,
        operation::{
            assume_role_with_saml::AssumeRoleWithSamlOutput,
            assume_role_with_web_identity::{
                builders::AssumeRoleWithWebIdentityFluentBuilder, AssumeRoleWithWebIdentityOutput,
            },
        },
        types::Credentials,
        Client,
    };

    pub const AWS_OIDC_AUDIENCE: &'static str = "sts.amazonaws.com";

    pub trait GetAuthenticationOutput {
        fn get_credentials(&self) -> Result<&Credentials>;
    }

    impl GetAuthenticationOutput for AssumeRoleWithSamlOutput {
        fn get_credentials(&self) -> Result<&Credentials> {
            let credentials = self.credentials.as_ref().ok_or(Error::BadGateway(
                "Error fetching credentials from AWS STS".to_string(),
            ))?;
            Ok(credentials)
        }
    }

    impl GetAuthenticationOutput for AssumeRoleWithWebIdentityOutput {
        fn get_credentials(&self) -> Result<&Credentials> {
            let credentials = self.credentials.as_ref().ok_or(Error::BadGateway(
                "Error fetching credentials from AWS STS".to_string(),
            ))?;
            Ok(credentials)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
    #[sqlx(type_name = "AWS_AUTH_RESOURCE_TYPE", rename_all = "lowercase")]
    #[serde(rename_all = "lowercase")]
    pub enum AwsAuthResourceType {
        Credentials,
        Oidc,
    }

    #[derive(Debug, Deserialize)]
    pub struct CredentialsAuth {
        #[serde(deserialize_with = "empty_as_none")]
        pub region: Option<String>,
        #[serde(rename = "awsAccessKeyId")]
        pub aws_access_key_id: String,
        #[serde(rename = "awsSecretAccessKey")]
        pub aws_secret_access_key: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct OidcAuth {
        #[serde(deserialize_with = "empty_as_none")]
        pub region: Option<String>,
        #[serde(rename = "roleArn")]
        pub role_arn: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub enum AWSAuthConfig {
        Credentials(CredentialsAuth),
        Oidc(OidcAuth),
    }

    pub async fn get_assume_role_with_web_identity_fluent_builder(
        oidc_auth: &OidcAuth,
        token: String,
        role_session_name: Option<impl ToString>,
    ) -> Result<AssumeRoleWithWebIdentityFluentBuilder> {
        let region = oidc_auth.region.as_deref().unwrap_or_else(|| "us-east-1");

        let credentials = AwsCredentials::new("", "", None, None, "UserInput");

        let config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(region.to_string()))
            .load()
            .await;

        let assume_role_with_web_identity_fluent_builder = Client::new(&config)
            .assume_role_with_web_identity()
            .set_role_arn(Some(oidc_auth.role_arn.to_owned()))
            .set_role_session_name(role_session_name.map(|str| str.to_string()))
            .set_web_identity_token(Some(token));

        Ok(assume_role_with_web_identity_fluent_builder)
    }
}
