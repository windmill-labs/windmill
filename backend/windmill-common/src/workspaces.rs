use async_recursion::async_recursion;
#[cfg(feature = "cloud")]
use backon::{ConstantBuilder, Retryable};
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use crate::{
    error::{self, to_anyhow, Error, Result},
    get_database_url,
    utils::get_custom_pg_instance_password,
    variables::{build_crypt, decrypt},
    PgDatabase, DB,
};

macro_rules! sqlx_bitflags {
    (
        $flags:ty => $repr:ty
    ) => {
        // ---- Type ----
        impl sqlx::Type<sqlx::Postgres> for $flags {
            fn type_info() -> sqlx::postgres::PgTypeInfo {
                <$repr as sqlx::Type<sqlx::Postgres>>::type_info()
            }
        }

        // ---- Encode ----
        impl<'q> sqlx::Encode<'q, sqlx::Postgres> for $flags {
            fn encode_by_ref(
                &self,
                buf: &mut sqlx::postgres::PgArgumentBuffer,
            ) -> std::result::Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
            {
                let bits: $repr = self.bits();
                <$repr as sqlx::Encode<sqlx::Postgres>>::encode(bits, buf)
            }
        }

        // ---- Decode ----
        impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $flags {
            fn decode(
                value: sqlx::postgres::PgValueRef<'r>,
            ) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
                let bits = <$repr as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
                <$flags>::from_bits(bits)
                    .ok_or_else(|| "invalid bitflags value from database".into())
            }
        }
    };
}

// Protection Rules - for fine-grained workspace access control

/// API representation of a protection rule
#[derive(Debug, Clone)]
pub struct ProtectionRuleset {
    pub workspace_id: String,
    pub name: String,
    pub rules: ProtectionRules,
    pub bypass_groups: Vec<String>,
    pub bypass_users: Vec<String>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    // #[sqlx(transparent)]
    pub struct ProtectionRules: i32 {
        const DISABLE_DIRECT_DEPLOYMENT =           1 << 0;
        const DISABLE_WORKSPACE_FORKING =           1 << 1;
    }
}

sqlx_bitflags!(ProtectionRules => i32);

#[derive(Serialize, Deserialize, strum_macros::EnumIter)]
pub enum ProtectionRuleKind {
    DisableDirectDeployment,
    DisableWorkspaceForking,
}

impl ProtectionRuleKind {
    pub const fn flag(&self) -> ProtectionRules {
        match self {
            ProtectionRuleKind::DisableDirectDeployment => {
                ProtectionRules::DISABLE_DIRECT_DEPLOYMENT
            }
            ProtectionRuleKind::DisableWorkspaceForking => {
                ProtectionRules::DISABLE_WORKSPACE_FORKING
            }
        }
    }

    pub const fn msg(&self) -> &str {
        match self {
            ProtectionRuleKind::DisableDirectDeployment => {
                "Cannot directly deploy in this workspace. Fork or Pull request required."
            }
            ProtectionRuleKind::DisableWorkspaceForking => "Forking this workspace is forbidden",
        }
    }
}

impl From<&Vec<ProtectionRuleKind>> for ProtectionRules {
    fn from(value: &Vec<ProtectionRuleKind>) -> Self {
        let mut r = ProtectionRules::empty();
        for rule in value {
            r = r | rule.flag();
        }
        r
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceGitSyncSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_type: Option<Vec<ObjectType>>,
    pub repositories: Vec<GitRepositorySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceDeploymentUISettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ObjectType {
    Script,
    Flow,
    App,
    Folder,
    Resource,
    Variable,
    Secret,
    Schedule,
    ResourceType,
    User,
    Group,
    Trigger,
    Settings,
    Key,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRepositorySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_types_override: Option<Vec<ObjectType>>,
    pub script_path: String,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
    pub group_by_folder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<GitSyncSettings>,
}

impl GitRepositorySettings {
    pub fn is_script_meets_min_version(&self, min_version: u32) -> error::Result<bool> {
        // example: "hub/28102/sync-script-to-git-repo-windmill"
        let current = self
            .script_path
            .split("/") // -> ["hub" "28102" "sync-script-to-git-repo-windmill"]
            .skip(1) // omit "hub"
            .next() // get numeric id
            .ok_or(Error::InternalErr(format!(
                "cannot get script version id from: {}",
                &self.script_path
            )))?
            .parse()
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "cannot get script version id from: {}. e: {e}",
                    &self.script_path
                );

                u32::MAX
            });

        Ok(current >= min_version) // this works on assumption that all scripts in hub have sequential ids
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitSyncSettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

impl Default for GitSyncSettings {
    fn default() -> Self {
        Self {
            include_path: Vec::new(),
            include_type: Vec::new(),
            exclude_path: None,
            extra_include_path: None,
        }
    }
}

#[derive(Clone)]
pub struct TeamPlanStatus {
    pub premium: bool,
    pub is_past_due: bool,
    pub max_tolerated_executions: Option<i32>,
}

lazy_static::lazy_static! {
    pub static ref TEAM_PLAN_CACHE: Cache<String, TeamPlanStatus> = Cache::new(5000);
    // Value: (rate_limit, cached_at_timestamp)
    pub static ref PUBLIC_APP_RATE_LIMIT_CACHE: Cache<String, (Option<i32>, i64)> = Cache::new(1000);
}

#[cfg(feature = "cloud")]
pub async fn get_team_plan_status(_db: &crate::DB, _w_id: &str) -> Result<TeamPlanStatus> {
    let cached = TEAM_PLAN_CACHE.get(_w_id);
    if let Some(cached) = cached {
        return Ok(cached);
    }

    let team_plan_info = (|| async {
        sqlx::query_as!(
            TeamPlanStatus,
            r#"
                SELECT
                    w.premium,
                    COALESCE(cw.is_past_due, false) as "is_past_due!",
                    cw.max_tolerated_executions
                FROM
                    workspace w
                    LEFT JOIN cloud_workspace_settings cw ON cw.workspace_id = w.id
                WHERE
                    w.id = $1
            "#,
            _w_id
        )
        .fetch_optional(_db)
        .await
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(5))
            .with_max_times(10),
    )
    .notify(|err, dur| {
        tracing::error!(
            "Failed to get team plan status for workspace {_w_id} (will retry in {dur:?}): {err:#}"
        );
    })
    .await
    .map_err(|err| {
        Error::internal_err(format!(
            "Failed to get team plan status for workspace {_w_id} after 10 retries: {err:#}"
        ))
    })?
    .unwrap_or_else(|| TeamPlanStatus {
        premium: false,
        is_past_due: false,
        max_tolerated_executions: None,
    });

    TEAM_PLAN_CACHE.insert(_w_id.to_string(), team_plan_info.clone());

    Ok(team_plan_info)
}

// Protection Rules Cache

lazy_static::lazy_static! {
    pub static ref PROTECTION_RULES_CACHE: Cache<String, (std::sync::Arc<Vec<ProtectionRuleset>>, i64)> = Cache::new(100);
}

/// Get all protection rules for a workspace with caching (60s TTL)
pub async fn get_protection_rules(
    workspace_id: &str,
    db: &DB,
) -> Result<std::sync::Arc<Vec<ProtectionRuleset>>> {
    let now = chrono::Utc::now().timestamp();

    // Check cache and expiry
    if let Some((cached_rules, expiry)) = PROTECTION_RULES_CACHE.get(workspace_id) {
        if expiry > now {
            return Ok(cached_rules);
        }
    }

    // Query database
    let rulesets = sqlx::query_as!(
        ProtectionRuleset,
        r#"
            SELECT
                workspace_id,
                name,
                rules as "rules: ProtectionRules",
                bypass_groups,
                bypass_users
            FROM workspace_protection_rule
            WHERE workspace_id = $1
            ORDER BY name
        "#,
        workspace_id
    )
    .fetch_all(db)
    .await
    .map_err(|e| Error::internal_err(format!("Failed to fetch protection rules: {}", e)))?;

    // Cache with 60s TTL
    let arc_rules = std::sync::Arc::new(rulesets);
    let expiry = now + 60;
    PROTECTION_RULES_CACHE.insert(workspace_id.to_string(), (arc_rules.clone(), expiry));

    Ok(arc_rules)
}

/// Invalidate the protection rules cache for a workspace
pub fn invalidate_protection_rules_cache(workspace_id: &str) {
    PROTECTION_RULES_CACHE.remove(workspace_id);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCheckResult {
    Allowed,
    Blocked(String),
}

/// Check if a user can bypass a protection rule
///
/// Returns `Allowed` if:
/// - User is in the rule's bypass users list (u/<username>)
/// - User's group is in the rule's bypass groups list (g/<groupname>)
///
/// Returns `Blocked` if:
/// - User is not in bypass lists
///
/// Returns `Err` if the rule is not found
pub async fn check_user_against_rule(
    workspace_id: &str,
    rule: &ProtectionRuleKind,
    username: &str,
    user_groups: &[String],
    is_admin: bool,
    db: &DB,
) -> Result<RuleCheckResult> {
    if is_admin {
        return Ok(RuleCheckResult::Allowed);
    }

    let rulesets = get_protection_rules(workspace_id, db).await?;

    for ruleset in rulesets.iter() {
        if ruleset.rules.contains(rule.flag()) {
            if ruleset.bypass_users.iter().any(|u| u == username)
                || ruleset
                    .bypass_groups
                    .iter()
                    .any(|g| user_groups.contains(g))
            {
                continue;
            }
            return Ok(RuleCheckResult::Blocked(format!(
                "Ruleset {} of {} blocked this action: {}",
                ruleset.name,
                workspace_id,
                rule.msg()
            )));
        }
    }

    Ok(RuleCheckResult::Allowed)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTable {
    pub database: DataTableDatabase,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DataTableDatabase {
    pub resource_type: DataTableCatalogResourceType,
    pub resource_path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum DataTableCatalogResourceType {
    #[strum(serialize = "postgres")]
    Postgresql,
    Instance,
}

pub async fn get_datatable_resource_from_db_unchecked(
    db: &DB,
    w_id: &str,
    name: &str,
) -> Result<serde_json::Value> {
    let datatable = sqlx::query_scalar!(
        r#"
            SELECT ws.datatable->'datatables'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting datatable {name}: {err}")))?
    .ok_or_else(|| Error::internal_err(format!("datatable {name} not found")))?;
    let datatable = serde_json::from_value::<DataTable>(datatable)?;

    let db_resource = if datatable.database.resource_type == DataTableCatalogResourceType::Instance
    {
        let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
        pg_creds.dbname = datatable.database.resource_path.clone();
        pg_creds.user = Some("custom_instance_user".to_string());
        pg_creds.password = Some(get_custom_pg_instance_password(&db).await?);
        serde_json::to_value(&pg_creds)
            .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?
    } else {
        transform_json_unchecked(
            &serde_json::Value::String(format!("$res:{}", datatable.database.resource_path)),
            w_id,
            db,
        )
        .await?
    };

    Ok(db_resource)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Ducklake {
    pub catalog: DucklakeCatalog,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeCatalog {
    pub resource_type: DucklakeCatalogResourceType,
    pub resource_path: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    pub path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum DucklakeCatalogResourceType {
    #[strum(serialize = "postgres")]
    Postgresql,
    Mysql,
    Instance,
}

#[derive(Deserialize, Serialize)]
pub struct DucklakeWithConnData {
    pub catalog: DucklakeCatalog,
    pub catalog_resource: serde_json::Value,
    pub storage: DucklakeStorage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

pub async fn get_ducklake_from_db_unchecked(
    name: &str,
    w_id: &str,
    db: &DB,
) -> Result<DucklakeWithConnData> {
    let ducklake = sqlx::query_scalar!(
        r#"
            SELECT ws.ducklake->'ducklakes'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting ducklake {name}: {err}")))?
    .ok_or_else(|| Error::internal_err(format!("ducklake {name} not found")))?;

    let ducklake = serde_json::from_value::<Ducklake>(ducklake)?;

    let catalog_resource =
        if ducklake.catalog.resource_type == DucklakeCatalogResourceType::Instance {
            let mut pg_creds = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
            pg_creds.dbname = ducklake.catalog.resource_path.clone();
            pg_creds.user = Some("custom_instance_user".to_string());
            pg_creds.password = Some(get_custom_pg_instance_password(&db).await?);
            serde_json::to_value(&pg_creds)
                .map_err(|e| Error::internal_err(format!("Error serializing pg creds: {}", e)))?
        } else {
            transform_json_unchecked(
                &serde_json::Value::String(format!("$res:{}", ducklake.catalog.resource_path)),
                w_id,
                db,
            )
            .await?
        };
    let ducklake = DucklakeWithConnData {
        catalog_resource,
        catalog: ducklake.catalog,
        storage: ducklake.storage,
        extra_args: ducklake.extra_args,
    };
    Ok(ducklake)
}

// This does not check for any permission. Should never be displayed to a user.
#[async_recursion]
async fn transform_json_unchecked(
    value: &serde_json::Value,
    w_id: &str,
    db: &DB,
) -> Result<serde_json::Value> {
    let value = match value {
        serde_json::Value::Object(map) => {
            let mut transformed_map = serde_json::Map::new();
            for (key, val) in map {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_map.insert(key.clone(), serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Object(transformed_map)
        }
        serde_json::Value::Array(arr) => {
            let mut transformed_array = Vec::new();
            for val in arr {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_array.push(serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Array(transformed_array)
        }
        serde_json::Value::String(s) if s.starts_with("$res:") => {
            let resource = sqlx::query_scalar!(
                "SELECT value AS \"value!: _\" FROM resource WHERE workspace_id = $1 AND path = $2",
                &w_id,
                &s[5..]
            )
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            transform_json_unchecked(&resource, w_id, db).await?
        }
        serde_json::Value::String(s) if s.starts_with("$var:") => {
            let variable = sqlx::query_scalar!(
                "SELECT value FROM variable WHERE workspace_id = $1 AND path = $2",
                &w_id,
                &s[5..]
            )
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            let mc = build_crypt(&db, &w_id).await?;
            let variable = decrypt(&mc, variable).map_err(|e| {
                Error::internal_err(format!("Error decrypting variable {}: {}", &s, e))
            })?;
            serde_json::Value::String(variable)
        }
        s @ serde_json::Value::String(_) => s.clone(),
        x => x.clone(),
    };

    Ok(value)
}
