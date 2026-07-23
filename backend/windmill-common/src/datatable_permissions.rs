//! Fine-grained, Postgres-enforced permissions for data tables (EE).
//!
//! When a data table has `permissions.enabled`, non-admin callers connect
//! through an ephemeral per-caller Postgres `LOGIN` role holding exactly the
//! privileges their grant statements resolve to, instead of the shared owner
//! role. Roles are real login roles with their own passwords — `SET ROLE` from
//! the shared connection would be escapable with `RESET ROLE` by user SQL.

use sha2::{Digest, Sha256};

use crate::{
    auth::is_super_admin_email,
    ee_oss::{get_license_plan, LicensePlan},
    error::{Error, Result},
    get_database_url,
    users::{PERMISSIONED_AS_GROUP_PREFIX, PERMISSIONED_AS_USER_PREFIX},
    utils::rd_string,
    variables::{build_crypt, decrypt, encrypt},
    workspaces::{
        datatable_shared_resource, get_datatable_config, DataTable, DataTableCatalogResourceType,
        DataTableFolderAccess, DataTableGrant,
    },
    PgDatabase, DB,
};

/// Reserved prefix for ephemeral data table roles. Any drop path MUST refuse
/// to touch a role that does not carry it.
pub const DATATABLE_EPHEMERAL_ROLE_PREFIX: &str = "wm_dt_";

pub const PERMISSIONED_AS_FOLDER_PREFIX: &str = "f/";

const EPHEMERAL_ROLE_CONNECTION_LIMIT: u32 = 25;
const CLEANUP_BATCH_SIZE: i64 = 5;

// ---------------------------------------------------------------------------
// Identifier helpers
// ---------------------------------------------------------------------------

/// Quote a Postgres identifier (`quote_ident` semantics).
pub fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn quote_literal(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Validate a schema/table name before it is stored in a grant statement.
/// Quoting makes any content safe to interpolate, but reject the cases that
/// can never be a valid Postgres identifier to fail at save time, not at
/// role-creation time.
pub fn validate_grant_identifier(kind: &str, s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(Error::BadRequest(format!("{kind} name cannot be empty")));
    }
    if s.len() > 63 {
        return Err(Error::BadRequest(format!(
            "{kind} name '{s}' exceeds Postgres's 63-byte identifier limit"
        )));
    }
    if s.contains('\0') {
        return Err(Error::BadRequest(format!(
            "{kind} name cannot contain NUL characters"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Role naming & permissions hash
// ---------------------------------------------------------------------------

fn sanitize_role_part(s: &str, max: usize) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .take(max)
        .collect()
}

/// Deterministic ephemeral role name for a caller on a data table. The
/// readable parts are truncated to fit Postgres's 63-byte identifier limit;
/// the trailing hash of the full `(workspace_id, permissioned_as, datatable)`
/// triple disambiguates truncated names. The data table is part of the
/// identity because bookkeeping is keyed by role name and grants are computed
/// per data table — one caller using two permissions-enabled data tables must
/// get two independent roles.
pub fn ephemeral_role_name(workspace_id: &str, permissioned_as: &str, datatable: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(permissioned_as.as_bytes());
    hasher.update([0u8]);
    hasher.update(datatable.as_bytes());
    let hash = hex::encode(&hasher.finalize()[..4]);

    let (kind, name) = permissioned_as
        .split_once('/')
        .unwrap_or(("x", permissioned_as));
    let kind = match kind {
        "u" => "u",
        "g" => "g",
        _ => "x",
    };
    // 6 (prefix) + 20 + 1 + 1 + 1 + 20 + 1 + 8 = 58 bytes max (all parts ASCII).
    format!(
        "{DATATABLE_EPHEMERAL_ROLE_PREFIX}{}_{kind}_{}_{hash}",
        sanitize_role_part(workspace_id, 20),
        sanitize_role_part(name, 20),
    )
}

/// Hash of everything the role's grants were derived from. Stored alongside
/// the role; any difference on later access triggers a drop-and-recreate, so
/// grant edits, membership changes and folder-perm changes take effect on the
/// next access without invalidation hooks.
pub fn perms_hash(
    workspace_id: &str,
    permissioned_as: &str,
    datatable: &str,
    db_identity: &str,
    matched: &[DataTableGrant],
    memberships: &[String],
) -> String {
    let mut canonical: Vec<DataTableGrant> = matched.to_vec();
    for g in canonical.iter_mut() {
        g.operations.sort();
        g.operations.dedup();
        g.tables.sort();
        g.tables.dedup();
    }
    canonical.sort();
    let mut memberships: Vec<&String> = memberships.iter().collect();
    memberships.sort();

    let mut hasher = Sha256::new();
    for part in [
        workspace_id,
        permissioned_as,
        datatable,
        db_identity,
        &serde_json::to_string(&canonical).unwrap_or_default(),
        &serde_json::to_string(&memberships).unwrap_or_default(),
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    hex::encode(hasher.finalize())
}

fn db_identity(is_instance: bool, owner: &PgDatabase) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        if is_instance { "instance" } else { "postgres" },
        owner.host,
        owner.port.unwrap_or(5432),
        owner.dbname,
        owner.user.as_deref().unwrap_or("")
    )
}

// ---------------------------------------------------------------------------
// Caller identity
// ---------------------------------------------------------------------------

/// Workspace admins and superadmins bypass fine-grained permissions and keep
/// using the shared role. `g/` callers are never admin.
pub async fn caller_is_admin(
    db: &DB,
    w_id: &str,
    permissioned_as: &str,
    permissioned_as_email: Option<&str>,
) -> Result<bool> {
    if let Some(email) = permissioned_as_email {
        if is_super_admin_email(db, email).await? {
            return Ok(true);
        }
    }
    if let Some(username) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
        return Ok(sqlx::query_scalar!(
            "SELECT is_admin FROM usr WHERE workspace_id = $1 AND username = $2",
            w_id,
            username
        )
        .fetch_optional(db)
        .await?
        .unwrap_or(false));
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Effective grant resolution
// ---------------------------------------------------------------------------

#[derive(PartialEq, PartialOrd)]
enum FolderAccessLevel {
    None,
    Read,
    Write,
}

fn folder_access_satisfies(
    level: &FolderAccessLevel,
    required: Option<&DataTableFolderAccess>,
) -> bool {
    match required {
        // A folder grant without an explicit level is treated as requiring
        // write — the most restrictive reading (save-time validation requires
        // the field, this only guards hand-edited configs).
        None | Some(DataTableFolderAccess::Write) => *level >= FolderAccessLevel::Write,
        Some(DataTableFolderAccess::Read) => *level >= FolderAccessLevel::Read,
    }
}

/// Resolve which grant statements apply to `permissioned_as` (`u/<user>` or
/// `g/<group>`). Returns the matched statements plus the group memberships
/// they were resolved through (folded into the perms hash so membership
/// changes invalidate the role).
pub async fn compute_effective_grants(
    db: &DB,
    w_id: &str,
    permissioned_as: &str,
    grants: &[DataTableGrant],
) -> Result<(Vec<DataTableGrant>, Vec<String>)> {
    let (username, groups): (Option<String>, Vec<String>) =
        if let Some(username) = permissioned_as.strip_prefix(PERMISSIONED_AS_USER_PREFIX) {
            let groups = sqlx::query_scalar!(
                "SELECT group_ FROM usr_to_group WHERE workspace_id = $1 AND usr = $2",
                w_id,
                username
            )
            .fetch_all(db)
            .await?;
            (Some(username.to_string()), groups)
        } else if let Some(group) = permissioned_as.strip_prefix(PERMISSIONED_AS_GROUP_PREFIX) {
            (None, vec![group.to_string()])
        } else {
            return Err(Error::BadRequest(format!(
                "Unexpected permissioned_as shape for data table access: {permissioned_as}"
            )));
        };

    // Load the folders referenced by folder-tenant statements in one query.
    let folder_names: Vec<String> = grants
        .iter()
        .filter_map(|g| g.tenant.strip_prefix(PERMISSIONED_AS_FOLDER_PREFIX))
        .map(str::to_string)
        .collect();
    let folders = if folder_names.is_empty() {
        vec![]
    } else {
        sqlx::query!(
            "SELECT name, owners, extra_perms FROM folder WHERE workspace_id = $1 AND name = ANY($2)",
            w_id,
            &folder_names
        )
        .fetch_all(db)
        .await?
    };

    let folder_level =
        |folder_name: &str| -> FolderAccessLevel {
            let Some(folder) = folders.iter().find(|f| f.name == folder_name) else {
                return FolderAccessLevel::None;
            };
            let group_keys: Vec<String> = groups
                .iter()
                .map(|g| format!("{PERMISSIONED_AS_GROUP_PREFIX}{g}"))
                .collect();
            let user_key = username
                .as_ref()
                .map(|u| format!("{PERMISSIONED_AS_USER_PREFIX}{u}"));
            // Owners have write; owners entries are `u/<user>` / `g/<group>`.
            if folder.owners.iter().any(|o| {
                user_key.as_deref() == Some(o.as_str()) || group_keys.iter().any(|k| k == o)
            }) {
                return FolderAccessLevel::Write;
            }
            let Some(extra_perms) = folder.extra_perms.as_object() else {
                return FolderAccessLevel::None;
            };
            // extra_perms value semantics: true = write, false = read.
            let mut level = FolderAccessLevel::None;
            for key in user_key.iter().chain(group_keys.iter()) {
                match extra_perms.get(key).and_then(|v| v.as_bool()) {
                    Some(true) => return FolderAccessLevel::Write,
                    Some(false) => level = FolderAccessLevel::Read,
                    None => {}
                }
            }
            level
        };

    let mut matched = vec![];
    for grant in grants {
        let applies =
            if let Some(folder_name) = grant.tenant.strip_prefix(PERMISSIONED_AS_FOLDER_PREFIX) {
                folder_access_satisfies(&folder_level(folder_name), grant.folder_access.as_ref())
            } else if let Some(group) = grant.tenant.strip_prefix(PERMISSIONED_AS_GROUP_PREFIX) {
                groups.iter().any(|g| g == group)
            } else {
                grant.tenant == permissioned_as
            };
        if applies {
            matched.push(grant.clone());
        }
    }

    Ok((matched, groups))
}

// ---------------------------------------------------------------------------
// Grant SQL generation
// ---------------------------------------------------------------------------

/// SQL applied on the target database, connected as the object owner (`GRANT`
/// requires ownership). For whole-schema statements, `ALTER DEFAULT
/// PRIVILEGES FOR ROLE <owner>` covers future tables — the `FOR ROLE` clause
/// is required because ADP is scoped to the creating role and all DDL funnels
/// through the owner via migrations.
pub fn grant_sql_statements(
    role: &str,
    owner_role: &str,
    statements: &[DataTableGrant],
) -> Vec<String> {
    let role_q = quote_ident(role);
    let owner_q = quote_ident(owner_role);
    let mut sql = vec![];
    for stmt in statements {
        let schema_q = quote_ident(&stmt.schema);
        let ops = {
            let mut ops: Vec<&str> = stmt.operations.iter().map(|o| o.as_ref()).collect();
            ops.sort();
            ops.dedup();
            ops.join(", ")
        };
        if ops.is_empty() {
            continue;
        }
        // Serial/identity columns need sequence usage for INSERT/UPDATE.
        let needs_sequences = stmt.operations.iter().any(|o| {
            matches!(
                o,
                crate::workspaces::DataTableOperation::Insert
                    | crate::workspaces::DataTableOperation::Update
            )
        });
        sql.push(format!("GRANT USAGE ON SCHEMA {schema_q} TO {role_q}"));
        if stmt.tables.is_empty() {
            sql.push(format!(
                "GRANT {ops} ON ALL TABLES IN SCHEMA {schema_q} TO {role_q}"
            ));
            sql.push(format!(
                "ALTER DEFAULT PRIVILEGES FOR ROLE {owner_q} IN SCHEMA {schema_q} GRANT {ops} ON TABLES TO {role_q}"
            ));
            if needs_sequences {
                sql.push(format!(
                    "GRANT USAGE ON ALL SEQUENCES IN SCHEMA {schema_q} TO {role_q}"
                ));
                sql.push(format!(
                    "ALTER DEFAULT PRIVILEGES FOR ROLE {owner_q} IN SCHEMA {schema_q} GRANT USAGE ON SEQUENCES TO {role_q}"
                ));
            }
        } else {
            for table in &stmt.tables {
                sql.push(format!(
                    "GRANT {ops} ON {schema_q}.{} TO {role_q}",
                    quote_ident(table)
                ));
            }
            if needs_sequences {
                sql.push(format!(
                    "GRANT USAGE ON ALL SEQUENCES IN SCHEMA {schema_q} TO {role_q}"
                ));
            }
        }
    }
    sql
}

// ---------------------------------------------------------------------------
// Role lifecycle
// ---------------------------------------------------------------------------

async fn connect_target(pg: &PgDatabase, db: &DB) -> Result<tokio_postgres::Client> {
    let (client, connection) = pg.connect(Some(db)).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::warn!("datatable ephemeral role target connection error: {e:#}");
        }
    });
    Ok(client)
}

fn pg_err(context: &str, e: tokio_postgres::Error) -> Error {
    // `Display` for tokio_postgres errors is just "db error" — the actual
    // Postgres message lives in the DbError.
    let detail = e
        .as_db_error()
        .map(|d| d.message().to_string())
        .unwrap_or_else(|| e.to_string());
    Error::internal_err(format!("{context}: {detail}"))
}

/// The revoke half of an instance-type role drop: database-level CONNECT was
/// granted by the main pool's user (the database owner), and only the grantor
/// (or a superuser) can revoke a privilege — `DROP OWNED` executed by
/// `custom_instance_user` fails on it. Best-effort: the grant may not exist.
async fn revoke_instance_connect(db: &DB, dbname: &str, role: &str) {
    if let Err(e) = sqlx::query(&format!(
        "REVOKE CONNECT ON DATABASE {} FROM {}",
        quote_ident(dbname),
        quote_ident(role)
    ))
    .execute(db)
    .await
    {
        tracing::warn!("revoking connect on {dbname} from {role}: {e:#}");
    }
}

/// `DROP OWNED BY` + `DROP ROLE`, refusing to drop anything without the
/// reserved prefix. `DROP OWNED` is required even though ephemeral roles own
/// no objects: `DROP ROLE` fails while any privilege (or default-privilege
/// entry) is still granted to the role. The membership self-grant makes
/// `DROP OWNED` work on PG16+ where CREATEROLE no longer implies it.
/// Instance-type callers must run [`revoke_instance_connect`] first.
async fn guarded_drop_role(client: &tokio_postgres::Client, role: &str) -> Result<()> {
    if !role.starts_with(DATATABLE_EPHEMERAL_ROLE_PREFIX) {
        return Err(Error::internal_err(format!(
            "Refusing to drop role '{role}': name does not start with the reserved '{DATATABLE_EPHEMERAL_ROLE_PREFIX}' prefix"
        )));
    }
    let role_q = quote_ident(role);
    let _ = client
        .batch_execute(&format!("GRANT {role_q} TO CURRENT_USER"))
        .await;
    client
        .batch_execute(&format!("DROP OWNED BY {role_q}"))
        .await
        .map_err(|e| pg_err(&format!("dropping privileges of role {role}"), e))?;
    client
        .batch_execute(&format!("DROP ROLE {role_q}"))
        .await
        .map_err(|e| pg_err(&format!("dropping role {role}"), e))?;
    Ok(())
}

async fn role_exists(client: &tokio_postgres::Client, role: &str) -> Result<bool> {
    let row = client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1)",
            &[&role],
        )
        .await
        .map_err(|e| Error::internal_err(format!("checking role existence: {e:#}")))?;
    Ok(row.get(0))
}

async fn create_role(
    db: &DB,
    client: &tokio_postgres::Client,
    role: &str,
    password: &str,
    is_instance: bool,
    dbname: &str,
) -> Result<()> {
    let create_sql = format!(
        "CREATE ROLE {} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION CONNECTION LIMIT {EPHEMERAL_ROLE_CONNECTION_LIMIT} PASSWORD {}",
        quote_ident(role),
        quote_literal(password)
    );
    match client.batch_execute(&create_sql).await {
        Ok(()) => Ok(()),
        // No `CREATE ROLE IF NOT EXISTS` exists — an out-of-band concurrent
        // creation surfaces as duplicate_object; drop it and retry once.
        Err(e) if e.code() == Some(&tokio_postgres::error::SqlState::DUPLICATE_OBJECT) => {
            if is_instance {
                revoke_instance_connect(db, dbname, role).await;
            }
            guarded_drop_role(client, role).await?;
            client
                .batch_execute(&create_sql)
                .await
                .map_err(|e| pg_err(&format!("creating role {role}"), e))
        }
        Err(e) => Err(pg_err(&format!("creating role {role}"), e)),
    }
}

/// One-time-per-database hardening for instance-type data tables. By default
/// PUBLIC has CONNECT on every database on the cluster, and any role can read
/// its own credentials from the script that uses them — so without this an
/// ephemeral role could connect to the main Windmill DB or another
/// workspace's data table DB. Executed on the main pool: database-level
/// REVOKE works from any DB on the cluster and the main pool's user owns
/// these databases. Revoking from PUBLIC does not affect explicitly-granted
/// roles, owners or superusers.
async fn harden_instance_databases(db: &DB, target_dbname: &str) -> Result<()> {
    let main_dbname = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?.dbname;
    for dbname in [target_dbname, main_dbname.as_str()] {
        sqlx::query(&format!(
            "REVOKE CONNECT ON DATABASE {} FROM PUBLIC",
            quote_ident(dbname)
        ))
        .execute(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "revoking PUBLIC connect on database {dbname}: {e:#}"
            ))
        })?;
    }
    Ok(())
}

/// Ensure the caller's ephemeral role exists with up-to-date grants and a
/// fresh sliding expiry. Returns `(role_name, cleartext_password)`.
pub async fn ensure_ephemeral_role(
    db: &DB,
    w_id: &str,
    datatable: &str,
    permissioned_as: &str,
    owner: &PgDatabase,
    is_instance: bool,
    matched: &[DataTableGrant],
    memberships: &[String],
) -> Result<(String, String)> {
    let role = ephemeral_role_name(w_id, permissioned_as, datatable);
    let hash = perms_hash(
        w_id,
        permissioned_as,
        datatable,
        &db_identity(is_instance, owner),
        matched,
        memberships,
    );

    // Fast path: role exists with current grants — refresh the sliding expiry.
    if let Some(encrypted) = sqlx::query_scalar!(
        "UPDATE datatable_ephemeral_role SET expires_at = now() + interval '5 minutes'
         WHERE role_name = $1 AND perms_hash = $2 AND expires_at > now()
         RETURNING password",
        &role,
        &hash
    )
    .fetch_optional(db)
    .await?
    {
        let mc = build_crypt(db, w_id).await?;
        return Ok((role, decrypt(&mc, encrypted)?));
    }

    // Slow path: (re)create the role under a per-role advisory lock on the
    // main DB (advisory locks are per-database — never take them on the
    // target cluster).
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('wm_dt_role:' || $1, 0))")
        .bind(&role)
        .execute(&mut *tx)
        .await?;

    // Double-checked locking: another resolution may have recreated the role
    // while we waited on the lock.
    let existing = sqlx::query!(
        "SELECT password, perms_hash, expires_at > now() AS \"fresh!\"
         FROM datatable_ephemeral_role WHERE role_name = $1",
        &role
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(row) = existing
        .as_ref()
        .filter(|r| r.fresh && r.perms_hash == hash)
    {
        let password = row.password.clone();
        sqlx::query!(
            "UPDATE datatable_ephemeral_role SET expires_at = now() + interval '5 minutes' WHERE role_name = $1",
            &role
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let mc = build_crypt(db, w_id).await?;
        return Ok((role, decrypt(&mc, password)?));
    }

    let password = rd_string(48);
    let mc = build_crypt(db, w_id).await?;
    let encrypted = encrypt(&mc, &password);

    let client = connect_target(owner, db).await?;
    if role_exists(&client, &role).await? {
        if is_instance {
            revoke_instance_connect(db, &owner.dbname, &role).await;
        }
        guarded_drop_role(&client, &role).await?;
    }
    create_role(db, &client, &role, &password, is_instance, &owner.dbname).await?;

    if is_instance {
        harden_instance_databases(db, &owner.dbname).await?;
        // The main pool's user owns instance databases; database-level GRANT
        // works from any DB on the cluster.
        sqlx::query(&format!(
            "GRANT CONNECT ON DATABASE {} TO {}",
            quote_ident(&owner.dbname),
            quote_ident(&role)
        ))
        .execute(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "granting connect on database {} to {role}: {e:#}",
                owner.dbname
            ))
        })?;
    } else {
        // On external databases the resource user may not own the database;
        // PUBLIC's default CONNECT privilege covers connection in that case.
        if let Err(e) = client
            .batch_execute(&format!(
                "GRANT CONNECT ON DATABASE {} TO {}",
                quote_ident(&owner.dbname),
                quote_ident(&role)
            ))
            .await
        {
            tracing::warn!(
                "could not grant CONNECT on external database {} to {role} (PUBLIC's default CONNECT usually covers it): {e:#}",
                owner.dbname
            );
        }
    }

    let owner_role = owner.user.as_deref().unwrap_or("postgres");
    for stmt in grant_sql_statements(&role, owner_role, matched) {
        client.batch_execute(&stmt).await.map_err(|e| {
            pg_err(
                &format!(
                    "applying data table grant `{stmt}` failed. The data table's owner role \
                     ('{owner_role}') must own the target schemas and tables — tables created \
                     through Windmill migrations are. Error"
                ),
                e,
            )
        })?;
    }
    drop(client);

    sqlx::query!(
        "INSERT INTO datatable_ephemeral_role
            (role_name, workspace_id, datatable, permissioned_as, password, perms_hash, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, now() + interval '5 minutes')
         ON CONFLICT (role_name) DO UPDATE SET
            workspace_id = EXCLUDED.workspace_id,
            datatable = EXCLUDED.datatable,
            permissioned_as = EXCLUDED.permissioned_as,
            password = EXCLUDED.password,
            perms_hash = EXCLUDED.perms_hash,
            expires_at = EXCLUDED.expires_at,
            created_at = now()",
        &role,
        w_id,
        datatable,
        permissioned_as,
        &encrypted,
        &hash
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Opportunistic cleanup of expired roles, bounded and best-effort.
    cleanup_expired_datatable_roles(db, CLEANUP_BATCH_SIZE).await;

    Ok((role, password))
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

/// Drop a bounded batch of expired ephemeral roles. Best-effort: contended
/// locks and roles with active sessions are skipped (a job may legitimately
/// outlive the TTL — the sliding expiry plus this check prevents dropping a
/// role mid-query). Stragglers are picked up by the next creation anywhere.
pub async fn cleanup_expired_datatable_roles(db: &DB, limit: i64) {
    let rows = match sqlx::query!(
        "SELECT role_name, workspace_id, datatable FROM datatable_ephemeral_role
         WHERE expires_at < now() LIMIT $1",
        limit
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("listing expired datatable ephemeral roles: {e:#}");
            return;
        }
    };
    for row in rows {
        if let Err(e) =
            cleanup_one_expired_role(db, &row.role_name, &row.workspace_id, &row.datatable).await
        {
            tracing::warn!(
                "cleaning up expired datatable ephemeral role {}: {e:#}",
                row.role_name
            );
        }
    }
}

async fn cleanup_one_expired_role(db: &DB, role: &str, w_id: &str, datatable: &str) -> Result<()> {
    if !role.starts_with(DATATABLE_EPHEMERAL_ROLE_PREFIX) {
        return Err(Error::internal_err(format!(
            "refusing to clean up role '{role}' without the reserved prefix"
        )));
    }
    let mut tx = db.begin().await?;
    let locked: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended('wm_dt_role:' || $1, 0))",
    )
    .bind(role)
    .fetch_one(&mut *tx)
    .await?;
    if !locked {
        return Ok(());
    }
    // Re-check under the lock: a concurrent resolution may have refreshed it.
    let still_expired = sqlx::query_scalar!(
        "SELECT expires_at < now() AS \"expired!\" FROM datatable_ephemeral_role WHERE role_name = $1",
        role
    )
    .fetch_optional(&mut *tx)
    .await?;
    if !still_expired.unwrap_or(false) {
        return Ok(());
    }

    match get_datatable_config(db, w_id, datatable).await {
        Ok(config) => {
            let owner: PgDatabase =
                serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
                    .map_err(|e| Error::internal_err(format!("parsing owner creds: {e}")))?;
            let client = connect_target(&owner, db).await?;
            let active: i64 = client
                .query_one(
                    "SELECT count(*) FROM pg_stat_activity WHERE usename = $1",
                    &[&role],
                )
                .await
                .map_err(|e| pg_err("checking active sessions", e))?
                .get(0);
            if active > 0 {
                return Ok(());
            }
            if role_exists(&client, role).await? {
                if config.database.resource_type == DataTableCatalogResourceType::Instance {
                    revoke_instance_connect(db, &owner.dbname, role).await;
                }
                guarded_drop_role(&client, role).await?;
            }
        }
        Err(_) => {
            // Data table config is gone — the target database is unknowable.
            // Try a bare drop on the main cluster (covers instance-type roles
            // whose database was already dropped); an orphaned role on an
            // external cluster stays inert (NOINHERIT, no grants worth
            // keeping) and is accepted.
            let _ = sqlx::query(&format!("DROP ROLE IF EXISTS {}", quote_ident(role)))
                .execute(db)
                .await;
        }
    }

    sqlx::query!(
        "DELETE FROM datatable_ephemeral_role WHERE role_name = $1",
        role
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Best-effort teardown of every ephemeral role of a data table (or a whole
/// workspace when `datatable` is `None`), used on data table deletion and
/// workspace deletion. Roles with active sessions are skipped; their
/// bookkeeping rows are removed regardless (workspace deletion cascades them
/// anyway) and the roles become inert stragglers.
pub async fn drop_datatable_ephemeral_roles_best_effort(
    db: &DB,
    w_id: &str,
    datatable: Option<&str>,
) {
    let rows = match sqlx::query!(
        "SELECT role_name, datatable FROM datatable_ephemeral_role
         WHERE workspace_id = $1 AND ($2::text IS NULL OR datatable = $2)",
        w_id,
        datatable
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("listing datatable ephemeral roles for teardown: {e:#}");
            return;
        }
    };
    for row in rows {
        if let Err(e) = teardown_role(db, w_id, &row.datatable, &row.role_name).await {
            tracing::warn!(
                "tearing down datatable ephemeral role {}: {e:#}",
                row.role_name
            );
        }
    }
}

async fn teardown_role(db: &DB, w_id: &str, datatable: &str, role: &str) -> Result<()> {
    if let Ok(config) = get_datatable_config(db, w_id, datatable).await {
        let owner: PgDatabase =
            serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
                .map_err(|e| Error::internal_err(format!("parsing owner creds: {e}")))?;
        let client = connect_target(&owner, db).await?;
        let active: i64 = client
            .query_one(
                "SELECT count(*) FROM pg_stat_activity WHERE usename = $1",
                &[&role],
            )
            .await
            .map_err(|e| pg_err("checking active sessions", e))?
            .get(0);
        if active == 0 && role_exists(&client, role).await? {
            if config.database.resource_type == DataTableCatalogResourceType::Instance {
                revoke_instance_connect(db, &owner.dbname, role).await;
            }
            guarded_drop_role(&client, role).await?;
        }
    }
    sqlx::query!(
        "DELETE FROM datatable_ephemeral_role WHERE role_name = $1",
        role
    )
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Checked resolver
// ---------------------------------------------------------------------------

pub fn datatable_permissions_enabled(datatable: &DataTable) -> bool {
    datatable
        .permissions
        .as_ref()
        .map(|p| p.enabled)
        .unwrap_or(false)
}

pub async fn datatable_license_valid() -> bool {
    matches!(get_license_plan().await, LicensePlan::Enterprise)
}

/// Permission-checked variant of `get_datatable_resource_from_db_unchecked`.
/// Admins (workspace admin via `usr.is_admin`, superadmin via the `password`
/// table) resolve to the shared owner role; other callers get ephemeral
/// per-caller credentials scoped to their effective grants, or a denial.
pub async fn get_datatable_resource_from_db_checked(
    db: &DB,
    w_id: &str,
    name: &str,
    permissioned_as: &str,
    permissioned_as_email: Option<&str>,
) -> Result<serde_json::Value> {
    let config = get_datatable_config(db, w_id, name).await?;
    if !datatable_permissions_enabled(&config) {
        return datatable_shared_resource(db, w_id, &config).await;
    }
    if caller_is_admin(db, w_id, permissioned_as, permissioned_as_email).await? {
        return datatable_shared_resource(db, w_id, &config).await;
    }
    // Fail closed on license loss: admins got through above and can disable
    // the feature; everyone else is denied rather than silently reverting to
    // the shared role.
    if !datatable_license_valid().await {
        return Err(Error::PermissionDenied(format!(
            "Data table '{name}' has fine-grained permissions enabled, which requires an \
             Enterprise license. Non-admin access is denied until a valid license is \
             configured or a workspace admin disables permissions on this data table."
        )));
    }
    let grants = config
        .permissions
        .as_ref()
        .map(|p| p.grants.as_slice())
        .unwrap_or(&[]);
    let (matched, memberships) =
        compute_effective_grants(db, w_id, permissioned_as, grants).await?;
    if matched.is_empty() {
        return Err(Error::PermissionDenied(format!(
            "You have no permissions on data table '{name}'. Ask a workspace admin to grant \
             you access in the data table's permission settings."
        )));
    }

    let mut pg: PgDatabase =
        serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
            .map_err(|e| Error::internal_err(format!("parsing data table owner creds: {e}")))?;
    let is_instance = config.database.resource_type == DataTableCatalogResourceType::Instance;
    let (role, password) = ensure_ephemeral_role(
        db,
        w_id,
        name,
        permissioned_as,
        &pg,
        is_instance,
        &matched,
        &memberships,
    )
    .await?;
    pg.user = Some(role);
    pg.password = Some(password);
    serde_json::to_value(&pg)
        .map_err(|e| Error::internal_err(format!("serializing ephemeral pg creds: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspaces::{DataTableFolderAccess, DataTableGrant, DataTableOperation};

    fn grant(
        tenant: &str,
        ops: &[DataTableOperation],
        schema: &str,
        tables: &[&str],
    ) -> DataTableGrant {
        DataTableGrant {
            tenant: tenant.to_string(),
            folder_access: None,
            operations: ops.to_vec(),
            schema: schema.to_string(),
            tables: tables.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn role_name_is_deterministic_prefixed_and_within_limit() {
        let a = ephemeral_role_name("my_workspace", "u/alice", "main");
        let b = ephemeral_role_name("my_workspace", "u/alice", "main");
        assert_eq!(a, b);
        assert!(a.starts_with(DATATABLE_EPHEMERAL_ROLE_PREFIX));
        assert!(a.len() <= 63);

        // Very long inputs still fit the 63-byte identifier limit and stay
        // distinct thanks to the trailing hash.
        let long_ws = "w".repeat(120);
        let long_user = format!("u/{}", "n".repeat(120));
        let long = ephemeral_role_name(&long_ws, &long_user, "main");
        assert!(long.len() <= 63);
        let long2 = ephemeral_role_name(&long_ws, &format!("{long_user}x"), "main");
        assert_ne!(long, long2);
    }

    #[test]
    fn role_name_distinguishes_caller_kind_workspace_and_datatable() {
        let base = ephemeral_role_name("ws", "u/team", "main");
        assert_ne!(base, ephemeral_role_name("ws", "g/team", "main"));
        assert_ne!(base, ephemeral_role_name("ws2", "u/team", "main"));
        assert_ne!(base, ephemeral_role_name("ws", "u/team", "other"));
    }

    #[test]
    fn perms_hash_is_order_insensitive_but_content_sensitive() {
        let g1 = grant(
            "u/alice",
            &[DataTableOperation::Select, DataTableOperation::Insert],
            "public",
            &["a", "b"],
        );
        let g2 = grant("g/eng", &[DataTableOperation::Delete], "public", &[]);

        let h_ab = perms_hash(
            "ws",
            "u/alice",
            "main",
            "instance|h|5432|db|owner",
            &[g1.clone(), g2.clone()],
            &["all".into(), "eng".into()],
        );
        // Statement order and membership order don't matter.
        let h_ba = perms_hash(
            "ws",
            "u/alice",
            "main",
            "instance|h|5432|db|owner",
            &[g2.clone(), g1.clone()],
            &["eng".into(), "all".into()],
        );
        assert_eq!(h_ab, h_ba);

        // Operation content does.
        let mut g1_less = g1.clone();
        g1_less.operations = vec![DataTableOperation::Select];
        let h_less = perms_hash(
            "ws",
            "u/alice",
            "main",
            "instance|h|5432|db|owner",
            &[g1_less, g2.clone()],
            &["all".into(), "eng".into()],
        );
        assert_ne!(h_ab, h_less);

        // Target database identity does.
        let h_other_db = perms_hash(
            "ws",
            "u/alice",
            "main",
            "instance|h|5432|other|owner",
            &[g1, g2],
            &["all".into(), "eng".into()],
        );
        assert_ne!(h_ab, h_other_db);
    }

    #[test]
    fn quote_ident_escapes_quotes() {
        assert_eq!(quote_ident("simple"), "\"simple\"");
        assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
    }

    #[test]
    fn grant_sql_per_table_and_sequences() {
        let sql = grant_sql_statements(
            "wm_dt_r",
            "custom_instance_user",
            &[grant(
                "u/alice",
                &[DataTableOperation::Select, DataTableOperation::Insert],
                "public",
                &["orders"],
            )],
        );
        assert_eq!(
            sql,
            vec![
                "GRANT USAGE ON SCHEMA \"public\" TO \"wm_dt_r\"".to_string(),
                "GRANT INSERT, SELECT ON \"public\".\"orders\" TO \"wm_dt_r\"".to_string(),
                "GRANT USAGE ON ALL SEQUENCES IN SCHEMA \"public\" TO \"wm_dt_r\"".to_string(),
            ]
        );
    }

    #[test]
    fn grant_sql_whole_schema_uses_default_privileges_for_owner() {
        let sql = grant_sql_statements(
            "wm_dt_r",
            "custom_instance_user",
            &[grant(
                "u/alice",
                &[DataTableOperation::Select],
                "public",
                &[],
            )],
        );
        assert_eq!(
            sql,
            vec![
                "GRANT USAGE ON SCHEMA \"public\" TO \"wm_dt_r\"".to_string(),
                "GRANT SELECT ON ALL TABLES IN SCHEMA \"public\" TO \"wm_dt_r\"".to_string(),
                "ALTER DEFAULT PRIVILEGES FOR ROLE \"custom_instance_user\" IN SCHEMA \"public\" GRANT SELECT ON TABLES TO \"wm_dt_r\"".to_string(),
            ]
        );
        // SELECT-only: no sequence grants.
        assert!(!sql.iter().any(|s| s.contains("SEQUENCES")));
    }

    #[test]
    fn folder_access_requirements() {
        use FolderAccessLevel::*;
        assert!(folder_access_satisfies(
            &Write,
            Some(&DataTableFolderAccess::Write)
        ));
        assert!(folder_access_satisfies(
            &Write,
            Some(&DataTableFolderAccess::Read)
        ));
        assert!(folder_access_satisfies(
            &Read,
            Some(&DataTableFolderAccess::Read)
        ));
        assert!(!folder_access_satisfies(
            &Read,
            Some(&DataTableFolderAccess::Write)
        ));
        assert!(!folder_access_satisfies(
            &None,
            Some(&DataTableFolderAccess::Read)
        ));
        // Missing selector on a folder grant defaults to requiring write.
        assert!(!folder_access_satisfies(&Read, Option::None));
        assert!(folder_access_satisfies(&Write, Option::None));
    }
}
