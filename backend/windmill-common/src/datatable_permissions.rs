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

/// Transaction-scoped advisory lock serializing a workspace's ephemeral role
/// creation with its deletion-time strict teardown. Bind the workspace id.
/// Lock order where both are taken: workspace lock first, then per-role lock.
pub const WORKSPACE_ROLES_LOCK: &str =
    "SELECT pg_advisory_xact_lock(hashtextextended('wm_dt_ws:' || $1::text, 0))";

/// Reserved prefix for the dedicated owner roles of protected instance
/// databases. Like the ephemeral prefix, no drop path may touch a role without
/// it.
pub const DATATABLE_OWNER_ROLE_PREFIX: &str = "wm_dto_";

/// Transaction-scoped advisory lock serializing (de)provisioning of one
/// protected instance database's owner role. Bind the database name.
const OWNER_ROLE_LOCK: &str =
    "SELECT pg_advisory_xact_lock(hashtextextended('wm_dto:' || $1::text, 0))";

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
///
/// Authorization: does not authenticate the supplied identity — callers MUST
/// pass a `permissioned_as` they have verified belongs to the caller (a job's
/// `permissioned_as`, or the authed user's own username).
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
// Dedicated owner role for protected instance databases
// ---------------------------------------------------------------------------

/// Deterministic owner-role name for a protected instance database.
pub fn instance_owner_role_name(dbname: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(dbname.as_bytes());
    let hash = hex::encode(&hasher.finalize()[..4]);
    format!(
        "{DATATABLE_OWNER_ROLE_PREFIX}{}_{hash}",
        sanitize_role_part(dbname, 40)
    )
}

/// Credentials of a protected instance database's dedicated owner role, if it
/// has been provisioned.
///
/// Authorization: returns a full-access login for the database and performs no
/// authorization — crate-internal on purpose; it must only ever be reached
/// through the resolvers that gate on admin-ness or effective grants.
pub(crate) async fn instance_owner_creds(db: &DB, dbname: &str) -> Result<Option<(String, String)>> {
    let Some(row) = sqlx::query!(
        "SELECT role_name, password, workspace_id FROM datatable_owner_role WHERE dbname = $1",
        dbname
    )
    .fetch_optional(db)
    .await?
    else {
        return Ok(None);
    };
    let password = decrypt_with_refresh(db, &row.workspace_id, row.password).await?;
    Ok(Some((row.role_name, password)))
}

/// Connect to an instance database as the main pool's user (owner of these
/// databases), which is what can reassign ownership between roles.
async fn connect_instance_db_as_superuser(db: &DB, dbname: &str) -> Result<tokio_postgres::Client> {
    let mut pg = PgDatabase::parse_uri(&get_database_url().await?.as_str().await)?;
    pg.dbname = dbname.to_string();
    connect_target(&pg, db).await
}

/// Make sure a protected instance database has its dedicated owner role: a
/// `LOGIN` role that owns the database's objects, can create the ephemeral
/// roles, and is never handed to non-admin SQL. `custom_instance_user` loses
/// CONNECT on the database, so a caller who learns that shared password from
/// an unprotected data table on the same cluster cannot reach this one.
/// Idempotent, and re-asserts the revoke on every call (superadmin re-running
/// the instance-database setup re-grants it).
pub async fn provision_instance_owner_role(db: &DB, w_id: &str, dbname: &str) -> Result<()> {
    let role = instance_owner_role_name(dbname);
    let mut tx = db.begin().await?;
    sqlx::query(OWNER_ROLE_LOCK)
        .bind(dbname)
        .execute(&mut *tx)
        .await?;

    let existing = sqlx::query_scalar!(
        "SELECT password FROM datatable_owner_role WHERE dbname = $1",
        dbname
    )
    .fetch_optional(&mut *tx)
    .await?;

    let client = connect_instance_db_as_superuser(db, dbname).await?;
    let role_present = role_exists(&client, &role).await?;

    // A row whose role vanished (manual cleanup, restored cluster) must be
    // rebuilt with a fresh password rather than trusted.
    let password = match (&existing, role_present) {
        (Some(encrypted), true) => decrypt_with_refresh(db, w_id, encrypted.clone()).await?,
        _ => {
            let password = rd_string(48);
            if role_present {
                client
                    .batch_execute(&format!(
                        "ALTER ROLE {} WITH LOGIN PASSWORD {}",
                        quote_ident(&role),
                        quote_literal(&password)
                    ))
                    .await
                    .map_err(|e| pg_err(&format!("resetting owner role {role} password"), e))?;
            } else {
                client
                    .batch_execute(&format!(
                        "CREATE ROLE {} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION PASSWORD {}",
                        quote_ident(&role),
                        quote_literal(&password)
                    ))
                    .await
                    .map_err(|e| pg_err(&format!("creating owner role {role}"), e))?;
            }
            password
        }
    };

    // Take over everything `custom_instance_user` owns in THIS database (the
    // statement is database-scoped), so the ephemeral-role grants and
    // `ALTER DEFAULT PRIVILEGES FOR ROLE <owner>` have an owner to hang off,
    // and give the role what migrations need to keep creating objects.
    client
        .batch_execute(&format!(
            "REASSIGN OWNED BY custom_instance_user TO {role};
             GRANT ALL ON SCHEMA public TO {role};",
            role = quote_ident(&role)
        ))
        .await
        .map_err(|e| pg_err(&format!("transferring ownership to {role}"), e))?;
    drop(client);

    // Database-level grants/revokes are executed from the main pool: any
    // database on the cluster can carry them and the main user owns these.
    sqlx::query(&format!(
        "GRANT CONNECT, CREATE ON DATABASE {} TO {}",
        quote_ident(dbname),
        quote_ident(&role)
    ))
    .execute(db)
    .await
    .map_err(|e| Error::internal_err(format!("granting {role} access to {dbname}: {e:#}")))?;
    // CDC connects as `custom_instance_replication_user`, which reaches these
    // databases only by inheriting `custom_instance_user` — the revoke below
    // would take it down with the parent. It is never handed to user SQL, so
    // a direct grant keeps triggers/capture working without reopening
    // anything.
    sqlx::query(&format!(
        "GRANT CONNECT ON DATABASE {} TO custom_instance_replication_user",
        quote_ident(dbname)
    ))
    .execute(db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "granting custom_instance_replication_user access to {dbname}: {e:#}"
        ))
    })?;

    // Read the cipher fresh under the lock: a key rotation may have committed
    // while this call waited, and a cached pre-rotation cipher would make the
    // stored owner password permanently undecryptable.
    crate::variables::WORKSPACE_CRYPT_CACHE.remove(w_id);
    let mc = build_crypt(db, w_id).await?;
    let encrypted = encrypt(&mc, &password);
    sqlx::query!(
        "INSERT INTO datatable_owner_role (dbname, role_name, password, workspace_id)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (dbname) DO UPDATE SET
            role_name = EXCLUDED.role_name,
            password = EXCLUDED.password,
            workspace_id = EXCLUDED.workspace_id",
        dbname,
        &role,
        &encrypted,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    // Commit the row BEFORE locking `custom_instance_user` out: if this call
    // dies in between, the fallback to the shared role still works and the
    // next call re-asserts the lockout. The reverse order would strand the
    // database with no usable credentials at all.
    tx.commit().await?;

    sqlx::query(&format!(
        "REVOKE CONNECT ON DATABASE {} FROM custom_instance_user",
        quote_ident(dbname)
    ))
    .execute(db)
    .await
    .map_err(|e| {
        Error::internal_err(format!(
            "revoking custom_instance_user access to {dbname}: {e:#}"
        ))
    })?;
    // Before PG16, CREATEROLE lets a role alter ANY non-superuser role — so a
    // caller who takes over `custom_instance_user` from an unprotected data
    // table could simply reset this database's owner-role password. Nothing
    // needs that attribute any more (instance role management runs as the main
    // pool user), so drop it cluster-wide once protection is in use.
    sqlx::query("ALTER ROLE custom_instance_user NOCREATEROLE")
        .execute(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "removing CREATEROLE from custom_instance_user: {e:#}"
            ))
        })?;
    Ok(())
}

/// Hand a protected instance database back to `custom_instance_user`: used
/// when permissions are disabled, the data table is deleted or re-pointed, and
/// on workspace deletion — the bookkeeping row must never outlive the
/// workspace key that decrypts its password.
pub async fn deprovision_instance_owner_role(db: &DB, dbname: &str) -> Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query(OWNER_ROLE_LOCK)
        .bind(dbname)
        .execute(&mut *tx)
        .await?;
    let Some(role) = sqlx::query_scalar!(
        "SELECT role_name FROM datatable_owner_role WHERE dbname = $1",
        dbname
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(());
    };
    if !role.starts_with(DATATABLE_OWNER_ROLE_PREFIX) {
        return Err(Error::internal_err(format!(
            "refusing to drop role '{role}': name does not start with the reserved '{DATATABLE_OWNER_ROLE_PREFIX}' prefix"
        )));
    }

    // The database may already be gone (dropped fork/instance database); the
    // row must still be cleared, and the role is then privilege-free.
    let db_exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1) AS \"e!\"",
        dbname
    )
    .fetch_one(db)
    .await?;
    if db_exists {
        let client = connect_instance_db_as_superuser(db, dbname).await?;
        client
            .batch_execute(&format!(
                "REASSIGN OWNED BY {role} TO custom_instance_user; DROP OWNED BY {role};",
                role = quote_ident(&role)
            ))
            .await
            .map_err(|e| pg_err(&format!("returning ownership from {role}"), e))?;
        drop(client);
        sqlx::query(&format!(
            "GRANT CONNECT ON DATABASE {} TO custom_instance_user",
            quote_ident(dbname)
        ))
        .execute(db)
        .await
        .map_err(|e| {
            Error::internal_err(format!(
                "restoring custom_instance_user access to {dbname}: {e:#}"
            ))
        })?;
        // The replication role goes back to inheriting the shared role.
        let _ = sqlx::query(&format!(
            "REVOKE CONNECT ON DATABASE {} FROM custom_instance_replication_user",
            quote_ident(dbname)
        ))
        .execute(db)
        .await;
    }
    sqlx::query(&format!("DROP ROLE IF EXISTS {}", quote_ident(&role)))
        .execute(db)
        .await
        .map_err(|e| Error::internal_err(format!("dropping owner role {role}: {e:#}")))?;
    sqlx::query!("DELETE FROM datatable_owner_role WHERE dbname = $1", dbname)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Best-effort [`deprovision_instance_owner_role`] for the paths where a
/// failure must not block the operation (config saves, database re-points).
pub async fn deprovision_instance_owner_role_best_effort(db: &DB, dbname: &str) {
    if let Err(e) = deprovision_instance_owner_role(db, dbname).await {
        tracing::warn!("deprovisioning owner role of instance database {dbname}: {e:#}");
    }
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
    // Also cover legacy instance databases provisioned before the revoke was
    // added to the setup paths — best-effort (a legacy entry may reference a
    // since-dropped database), scoped to when the feature is actually in use.
    let registered: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT jsonb_object_keys(value->'databases') FROM global_settings
         WHERE name = 'custom_instance_pg_databases'",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for dbname in registered
        .iter()
        .map(String::as_str)
        .filter(|d| *d != target_dbname && *d != main_dbname)
    {
        if let Err(e) = sqlx::query(&format!(
            "REVOKE CONNECT ON DATABASE {} FROM PUBLIC",
            quote_ident(dbname)
        ))
        .execute(db)
        .await
        {
            tracing::warn!("revoking PUBLIC connect on instance database {dbname}: {e:#}");
        }
    }
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

/// Owner credentials + type of the cluster a role was created on, stored
/// workspace-key-encrypted in the bookkeeping row. Without it, re-pointing a
/// data table's resource at a different cluster would strand the role on the
/// old cluster with its grants intact and no way to reach it again.
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredOwnerTarget {
    pg: PgDatabase,
    is_instance: bool,
}

fn same_target(a: &PgDatabase, b: &PgDatabase) -> bool {
    a.host == b.host && a.port == b.port && a.dbname == b.dbname && a.user == b.user
}

enum DropOutcome {
    Dropped,
    SkippedActive,
}

/// Drop the role on the given target, or — when sessions are still active —
/// strip its ability to log back in and leave the drop to a later sweep.
async fn drop_or_disable_on_target(
    db: &DB,
    owner: &PgDatabase,
    is_instance: bool,
    role: &str,
) -> Result<DropOutcome> {
    // On instance databases act as the main pool's user: a `CREATEROLE` role
    // may only drop roles it created itself, so the dedicated owner role
    // cannot clean up roles minted before an ownership transfer (or by a
    // previous owner). The main user has authority over the whole cluster.
    // External databases keep using the resource's own user, which created
    // its roles and can therefore drop them.
    let client = if is_instance {
        connect_instance_db_as_superuser(db, &owner.dbname).await?
    } else {
        connect_target(owner, db).await?
    };
    let active: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_stat_activity WHERE usename = $1",
            &[&role],
        )
        .await
        .map_err(|e| pg_err("checking active sessions", e))?
        .get(0);
    if active > 0 {
        disable_role_login(&client, role).await?;
        return Ok(DropOutcome::SkippedActive);
    }
    if role_exists(&client, role).await? {
        if is_instance {
            revoke_instance_connect(db, &owner.dbname, role).await;
        }
        guarded_drop_role(&client, role).await?;
    }
    Ok(DropOutcome::Dropped)
}

/// Decrypt with the cached workspace cipher, refreshing the cache once on
/// failure: around a key rotation a process can hold the previous cipher
/// cached for up to its TTL, and treating that as corruption would make
/// revocation lose its pointer to a recorded target.
async fn decrypt_with_refresh(db: &DB, w_id: &str, value: String) -> Result<String> {
    let mc = build_crypt(db, w_id).await?;
    match decrypt(&mc, value.clone()) {
        Ok(v) => Ok(v),
        Err(_) => {
            crate::variables::WORKSPACE_CRYPT_CACHE.remove(w_id);
            let mc = build_crypt(db, w_id).await?;
            decrypt(&mc, value)
        }
    }
}

/// Decode a bookkeeping row's stored owner target, if any.
async fn decode_stored_target(
    db: &DB,
    w_id: &str,
    owner_creds: Option<String>,
) -> Option<StoredOwnerTarget> {
    let encrypted = owner_creds?;
    let json = decrypt_with_refresh(db, w_id, encrypted).await.ok()?;
    serde_json::from_str(&json).ok()
}

/// Ensure the caller's ephemeral role exists with up-to-date grants and a
/// fresh sliding expiry. Returns ready-to-use connection credentials.
/// Deliberately private: live credentials must only flow out through
/// [`get_datatable_resource_from_db_checked`], which performs the
/// authorization this function assumes already happened. `fast_path_hash` is
/// the caller's grant hash, only trusted for the lock-free refresh; the slow
/// path re-derives config, grants and target under the locks so a role is
/// never created from state that a concurrent edit or deletion invalidated.
async fn ensure_ephemeral_role(
    db: &DB,
    w_id: &str,
    datatable: &str,
    permissioned_as: &str,
    fast_path_hash: &str,
) -> Result<PgDatabase> {
    let role = ephemeral_role_name(w_id, permissioned_as, datatable);

    // Fast path: role exists with current grants — refresh the sliding expiry.
    if let Some(row) = sqlx::query!(
        "UPDATE datatable_ephemeral_role SET expires_at = now() + interval '5 minutes'
         WHERE role_name = $1 AND perms_hash = $2 AND expires_at > now()
         RETURNING password, owner_creds",
        &role,
        fast_path_hash
    )
    .fetch_optional(db)
    .await?
    {
        let password = decrypt_with_refresh(db, w_id, row.password).await?;
        // Serve the CURRENTLY configured connection settings: TLS/verification
        // edits on the resource must apply on next access even though they
        // don't change the physical identity in the perms hash. A recorded
        // target on a different physical database means a re-point is pending
        // — fall through to the slow path, which revokes there first.
        let stored = decode_stored_target(db, w_id, row.owner_creds).await;
        let config = get_datatable_config(db, w_id, datatable).await?;
        let current: PgDatabase =
            serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
                .map_err(|e| Error::internal_err(format!("parsing data table owner creds: {e}")))?;
        if let Some(stored) = stored {
            if same_target(&stored.pg, &current) {
                let mut pg = current;
                pg.user = Some(role);
                pg.password = Some(password);
                return Ok(pg);
            }
        }
        // Re-point pending or legacy row without a recorded target: rebuild
        // through the slow path.
    }

    // Slow path: (re)create the role under a per-role advisory lock on the
    // main DB (advisory locks are per-database — never take them on the
    // target cluster). The workspace-scoped lock (taken first — same order as
    // workspace deletion, which holds it from strict teardown through its
    // commit) keeps a creation from racing workspace deletion: a role created
    // after deletion's teardown scan would be orphaned forever once the
    // bookkeeping rows and workspace key cascade away.
    let mut tx = db.begin().await?;
    sqlx::query(WORKSPACE_ROLES_LOCK)
        .bind(w_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('wm_dt_role:' || $1, 0))")
        .bind(&role)
        .execute(&mut *tx)
        .await?;

    // Everything the role is built from is re-derived under the locks: the
    // caller's reads happened before them, so a concurrent permission/config
    // edit (serialized through these locks) or workspace deletion may have
    // changed or removed what the role must reflect. Config gone (data table
    // or workspace deleted) errors out before any external side effect.
    let config = get_datatable_config(db, w_id, datatable).await?;
    let grants = config
        .permissions
        .as_ref()
        .filter(|p| p.enabled)
        .map(|p| p.grants.as_slice())
        .ok_or_else(|| {
            Error::internal_err(format!(
                "Permissions of data table '{datatable}' were disabled concurrently; retry."
            ))
        })?;
    let (matched, memberships) =
        compute_effective_grants(db, w_id, permissioned_as, grants).await?;
    if matched.is_empty() {
        return Err(Error::PermissionDenied(format!(
            "You have no permissions on data table '{datatable}'. Ask a workspace admin to \
             grant you access in the data table's permission settings."
        )));
    }
    let is_instance = config.database.resource_type == DataTableCatalogResourceType::Instance;
    // Protected instance databases must be owned by their dedicated role
    // before any ephemeral role is minted: `custom_instance_user` is exposed
    // to non-admin SQL on unprotected data tables of the same cluster, so
    // leaving it able to CONNECT here would make the grants bypassable. This
    // is idempotent and also re-asserts the revoke, so a data table whose
    // permissions were enabled without provisioning (EE default on creation,
    // instance database created later) is healed on first non-admin access.
    if is_instance {
        provision_instance_owner_role(db, w_id, &config.database.resource_path).await?;
    }
    let owner: PgDatabase =
        serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
            .map_err(|e| Error::internal_err(format!("parsing data table owner creds: {e}")))?;
    let owner = &owner;
    let hash = perms_hash(
        w_id,
        permissioned_as,
        datatable,
        &db_identity(is_instance, owner),
        &matched,
        &memberships,
    );

    // Double-checked locking: another resolution may have recreated the role
    // while we waited on the lock.
    let existing = sqlx::query!(
        "SELECT password, perms_hash, owner_creds, expires_at > now() AS \"fresh!\"
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
        let password = decrypt_with_refresh(db, w_id, password).await?;
        let mut pg = owner.clone();
        pg.user = Some(role);
        pg.password = Some(password);
        return Ok(pg);
    }

    let password = rd_string(48);
    // Encrypt with a cipher read fresh under the locks: a concurrent key
    // rotation (which holds the same locks) may have committed while this
    // resolution waited, and a cached pre-rotation cipher would write
    // ciphertext that nothing can decrypt afterwards.
    crate::variables::WORKSPACE_CRYPT_CACHE.remove(w_id);
    let mc = build_crypt(db, w_id).await?;
    let encrypted = encrypt(&mc, &password);

    // If the data table's database moved since the role was created (resource
    // edit, config re-point), the role still exists on the PREVIOUS cluster
    // with its old grants — revoke it there first. Failure is fatal, not
    // logged: continuing would overwrite the stored target with the new
    // cluster's and lose the only pointer to the unrevoked role. Failing
    // keeps the row (old target intact) so the next access or sweep retries.
    let stored_creds = existing.and_then(|r| r.owner_creds);
    if let Some(stored) = decode_stored_target(db, w_id, stored_creds).await {
        if !same_target(&stored.pg, owner) {
            drop_or_disable_on_target(db, &stored.pg, stored.is_instance, &role)
                .await
                .map_err(|e| {
                    Error::internal_err(format!(
                        "cannot revoke this data table's previous credentials on its former \
                         database ({}/{}): {e:#}. Access stays blocked until that database is \
                         reachable; if it is permanently gone, ask an admin to clear the \
                         datatable_ephemeral_role entry.",
                        stored.pg.host, stored.pg.dbname
                    ))
                })?;
        }
    }

    let client = connect_target(owner, db).await?;
    // Role management on instance databases runs as the main pool's user (see
    // `drop_or_disable_on_target`); grants below still run as the owner, which
    // is what must appear in `ALTER DEFAULT PRIVILEGES FOR ROLE`.
    let role_admin = if is_instance {
        connect_instance_db_as_superuser(db, &owner.dbname).await?
    } else {
        connect_target(owner, db).await?
    };
    if role_exists(&role_admin, &role).await? {
        if is_instance {
            revoke_instance_connect(db, &owner.dbname, &role).await;
        }
        guarded_drop_role(&role_admin, &role).await?;
    }
    create_role(
        db,
        &role_admin,
        &role,
        &password,
        is_instance,
        &owner.dbname,
    )
    .await?;

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
    for stmt in grant_sql_statements(&role, owner_role, &matched) {
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

    let owner_creds = encrypt(
        &mc,
        &serde_json::to_string(&StoredOwnerTarget { pg: owner.clone(), is_instance })
            .map_err(|e| Error::internal_err(format!("serializing owner target: {e}")))?,
    );
    sqlx::query!(
        "INSERT INTO datatable_ephemeral_role
            (role_name, workspace_id, datatable, permissioned_as, password, perms_hash, owner_creds, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, now() + interval '5 minutes')
         ON CONFLICT (role_name) DO UPDATE SET
            workspace_id = EXCLUDED.workspace_id,
            datatable = EXCLUDED.datatable,
            permissioned_as = EXCLUDED.permissioned_as,
            password = EXCLUDED.password,
            perms_hash = EXCLUDED.perms_hash,
            owner_creds = EXCLUDED.owner_creds,
            expires_at = EXCLUDED.expires_at,
            created_at = now()",
        &role,
        w_id,
        datatable,
        permissioned_as,
        &encrypted,
        &hash,
        &owner_creds
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Opportunistic cleanup of expired roles, bounded and best-effort.
    cleanup_expired_datatable_roles(db, CLEANUP_BATCH_SIZE).await;

    let mut pg = owner.clone();
    pg.user = Some(role);
    pg.password = Some(password);
    Ok(pg)
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
         WHERE expires_at < now() ORDER BY expires_at LIMIT $1",
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
        let outcome =
            cleanup_one_expired_role(db, &row.role_name, &row.workspace_id, &row.datatable).await;
        if let Err(e) = &outcome {
            tracing::warn!(
                "cleaning up expired datatable ephemeral role {}: {e:#}",
                row.role_name
            );
        }
        // Push the row's expiry forward unless it was actually reaped: a
        // persistently failing target (unreachable database) or a role holding
        // long-lived sessions must not monopolize every sweep batch and starve
        // later expired roles. Skipped roles are already NOLOGIN, so the delay
        // costs no privilege exposure.
        if !matches!(outcome, Ok(CleanupOutcome::Reaped)) {
            let _ = sqlx::query!(
                "UPDATE datatable_ephemeral_role SET expires_at = now() + interval '5 minutes'
                 WHERE role_name = $1 AND expires_at < now()",
                &row.role_name
            )
            .execute(db)
            .await;
        }
    }
}

enum CleanupOutcome {
    /// Role dropped and its bookkeeping row deleted.
    Reaped,
    /// Left for a later sweep (lock contention, still-fresh row, or active
    /// sessions — in the last case the role has been made NOLOGIN).
    Deferred,
}

async fn cleanup_one_expired_role(
    db: &DB,
    role: &str,
    w_id: &str,
    datatable: &str,
) -> Result<CleanupOutcome> {
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
        return Ok(CleanupOutcome::Deferred);
    }
    // Re-check under the lock: a concurrent resolution may have refreshed it.
    let row = sqlx::query!(
        "SELECT owner_creds, expires_at < now() AS \"expired!\" FROM datatable_ephemeral_role WHERE role_name = $1",
        role
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row.filter(|r| r.expired) else {
        return Ok(CleanupOutcome::Deferred);
    };

    match resolve_role_target(db, w_id, datatable, row.owner_creds).await {
        Some((owner, is_instance)) => {
            if matches!(
                drop_or_disable_with_instance_fallback(db, &owner, is_instance, role).await?,
                DropOutcome::SkippedActive
            ) {
                return Ok(CleanupOutcome::Deferred);
            }
        }
        None => {
            // Neither stored target nor config is usable — the target database
            // is unknowable. Try a bare drop on the main cluster (covers
            // instance-type roles whose database was already dropped); an
            // orphaned role on an external cluster stays inert (NOINHERIT, no
            // grants worth keeping) and is accepted.
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
    Ok(CleanupOutcome::Reaped)
}

/// One bookkeeping row captured before a permissions/config edit commits.
/// The post-commit teardown only revokes roles still carrying the captured
/// hash: a role concurrently recreated from the NEW config has a different
/// hash and must keep working.
pub struct RoleSnapshot {
    role_name: String,
    datatable: String,
    perms_hash: String,
}

/// Capture the current roles of a data table (or whole workspace) with their
/// perms hashes. Take this BEFORE committing the edit that invalidates them,
/// under [`WORKSPACE_ROLES_LOCK`] (so no role can be created between the
/// snapshot and the commit), then pass it to
/// [`teardown_snapshot_roles_best_effort`] after the commit.
///
/// Authorization: reads role bookkeeping metadata; call only from admin-gated
/// flows.
pub async fn snapshot_datatable_roles(
    db: &DB,
    w_id: &str,
    datatable: Option<&str>,
) -> Result<Vec<RoleSnapshot>> {
    Ok(sqlx::query!(
        "SELECT role_name, datatable, perms_hash FROM datatable_ephemeral_role
         WHERE workspace_id = $1 AND ($2::text IS NULL OR datatable = $2)",
        w_id,
        datatable
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .map(|r| RoleSnapshot {
        role_name: r.role_name,
        datatable: r.datatable,
        perms_hash: r.perms_hash,
    })
    .collect())
}

/// Best-effort teardown of a pre-edit role generation, used after permission
/// edits, data table deletion/renames and database re-points commit. Roles
/// that cannot be dropped keep their bookkeeping row (with the recorded
/// target), so the expiry sweep retries them — failure here loses nothing.
///
/// Authorization: mutates cluster roles; call only from admin-gated flows.
pub async fn teardown_snapshot_roles_best_effort(db: &DB, w_id: &str, snapshot: Vec<RoleSnapshot>) {
    for s in snapshot {
        if let Err(e) =
            teardown_role(db, w_id, &s.datatable, &s.role_name, Some(&s.perms_hash)).await
        {
            tracing::warn!(
                "tearing down datatable ephemeral role {}: {e:#}",
                s.role_name
            );
        }
    }
}

/// Strict teardown of every role of a workspace, for workspace deletion: the
/// bookkeeping rows (and the workspace key their targets are encrypted with)
/// are about to cascade away, so every role must be revoked NOW — dropped, or
/// at least stripped of LOGIN when sessions are still active. Any failure
/// (e.g. an unreachable external cluster) must abort the deletion; proceeding
/// would orphan a live LOGIN role with no remaining way to ever revoke it.
/// Callers must hold [`WORKSPACE_ROLES_LOCK`] until the deletion commits so
/// no new role is created after this scan.
///
/// Authorization: mutates cluster roles; call only from admin-gated flows.
pub async fn teardown_datatable_roles_strict(db: &DB, w_id: &str) -> Result<()> {
    let rows = sqlx::query!(
        "SELECT role_name, datatable FROM datatable_ephemeral_role WHERE workspace_id = $1",
        w_id
    )
    .fetch_all(db)
    .await?;
    let mut failures: Vec<String> = vec![];
    for row in rows {
        if let Err(e) = teardown_role(db, w_id, &row.datatable, &row.role_name, None).await {
            failures.push(format!("{} ({}): {e:#}", row.role_name, row.datatable));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::internal_err(format!(
            "could not revoke data table role(s): {}",
            failures.join("; ")
        )))
    }
}

/// Strip a live role's ability to open new connections. An ordinary role can
/// `ALTER ROLE CURRENT_USER PASSWORD ...` — so a caller keeping a session open
/// could otherwise reconnect with a self-chosen password long after
/// revocation. Only CREATEROLE can restore LOGIN, so this closes the
/// reconnect vector while letting in-flight queries finish; the kept
/// bookkeeping row makes the expiry sweep finish the drop later.
/// Failure is propagated: callers (notably strict workspace-deletion
/// teardown) must not treat an active role as revoked when it still holds
/// LOGIN.
async fn disable_role_login(client: &tokio_postgres::Client, role: &str) -> Result<()> {
    if !role.starts_with(DATATABLE_EPHEMERAL_ROLE_PREFIX) {
        return Err(Error::internal_err(format!(
            "refusing to alter role '{role}' without the reserved prefix"
        )));
    }
    client
        .batch_execute(&format!(
            "ALTER ROLE {} NOLOGIN CONNECTION LIMIT 0 PASSWORD NULL",
            quote_ident(role)
        ))
        .await
        .map_err(|e| pg_err(&format!("disabling login of ephemeral role {role}"), e))
}

/// [`drop_or_disable_on_target`], with a recovery path for instance targets
/// whose database no longer exists (fork database dropped, superadmin
/// instance-DB drop): connecting would fail forever and wedge revocation —
/// but roles are cluster-wide on the main cluster and a vanished database
/// takes every privilege granted in it along, so a bare guarded drop from the
/// main pool is a complete revocation there.
async fn drop_or_disable_with_instance_fallback(
    db: &DB,
    owner: &PgDatabase,
    is_instance: bool,
    role: &str,
) -> Result<DropOutcome> {
    match drop_or_disable_on_target(db, owner, is_instance, role).await {
        Ok(outcome) => Ok(outcome),
        Err(e) if is_instance => {
            let db_exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_database WHERE datname = $1) AS \"e!\"",
                &owner.dbname
            )
            .fetch_one(db)
            .await?;
            if db_exists {
                return Err(e);
            }
            if !role.starts_with(DATATABLE_EPHEMERAL_ROLE_PREFIX) {
                return Err(Error::internal_err(format!(
                    "refusing to drop role '{role}' without the reserved prefix"
                )));
            }
            sqlx::query(&format!("DROP ROLE IF EXISTS {}", quote_ident(role)))
                .execute(db)
                .await
                .map_err(|e| {
                    Error::internal_err(format!("dropping role {role} on main cluster: {e:#}"))
                })?;
            Ok(DropOutcome::Dropped)
        }
        Err(e) => Err(e),
    }
}

/// Where a role must be revoked: the stored owner target from its bookkeeping
/// row when present (survives resource/config re-points), else the currently
/// configured database. When the stored and configured targets are the same
/// physical database, the configured credentials win — a password-only
/// rotation of the resource must not leave revocation retrying obsolete
/// credentials forever.
async fn resolve_role_target(
    db: &DB,
    w_id: &str,
    datatable: &str,
    owner_creds: Option<String>,
) -> Option<(PgDatabase, bool)> {
    let stored = decode_stored_target(db, w_id, owner_creds).await;
    let current = match get_datatable_config(db, w_id, datatable).await {
        Ok(config) => {
            let is_instance =
                config.database.resource_type == DataTableCatalogResourceType::Instance;
            datatable_shared_resource(db, w_id, &config)
                .await
                .ok()
                .and_then(|v| serde_json::from_value::<PgDatabase>(v).ok())
                .map(|owner| (owner, is_instance))
        }
        Err(_) => None,
    };
    match (stored, current) {
        (Some(stored), Some((current, current_is_instance))) => {
            if same_target(&stored.pg, &current) {
                Some((current, current_is_instance))
            } else {
                Some((stored.pg, stored.is_instance))
            }
        }
        (Some(stored), None) => Some((stored.pg, stored.is_instance)),
        (None, current) => current,
    }
}

async fn teardown_role(
    db: &DB,
    w_id: &str,
    datatable: &str,
    role: &str,
    only_if_hash: Option<&str>,
) -> Result<()> {
    // Same per-role lock as role creation and the expiry sweep: without it,
    // teardown could observe a stale role while a resolver is recreating it
    // and drop the fresh role right before its bookkeeping row lands, leaving
    // callers with credentials for a nonexistent role.
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('wm_dt_role:' || $1, 0))")
        .bind(role)
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query!(
        "SELECT owner_creds, perms_hash FROM datatable_ephemeral_role WHERE role_name = $1",
        role
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let (Some(expected), Some(row)) = (only_if_hash, row.as_ref()) {
        // The role was recreated from the post-edit config while we were
        // getting here — it is current, not the generation this teardown
        // targets. Leave it alone.
        if row.perms_hash != expected {
            return Ok(());
        }
    }
    let owner_creds = row.and_then(|r| r.owner_creds);
    if let Some((owner, is_instance)) = resolve_role_target(db, w_id, datatable, owner_creds).await
    {
        if matches!(
            drop_or_disable_with_instance_fallback(db, &owner, is_instance, role).await?,
            DropOutcome::SkippedActive
        ) {
            // Keep the bookkeeping row: this teardown runs on grant revocation,
            // and deleting the row would permanently orphan a live role that
            // still holds its old grants (and whose password the caller
            // already has). With the row intact, the expiry sweep retries the
            // drop once the sessions are gone; meanwhile new logins are cut.
            // Expire it so the lock-free fast path stops handing out
            // credentials for the now-NOLOGIN role.
            sqlx::query!(
                "UPDATE datatable_ephemeral_role SET expires_at = now() WHERE role_name = $1",
                role
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(());
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

    // This hash is only a fast-path key; the slow path re-derives everything
    // under its locks.
    let owner: PgDatabase =
        serde_json::from_value(datatable_shared_resource(db, w_id, &config).await?)
            .map_err(|e| Error::internal_err(format!("parsing data table owner creds: {e}")))?;
    let is_instance = config.database.resource_type == DataTableCatalogResourceType::Instance;
    let fast_path_hash = perms_hash(
        w_id,
        permissioned_as,
        name,
        &db_identity(is_instance, &owner),
        &matched,
        &memberships,
    );
    let pg = ensure_ephemeral_role(db, w_id, name, permissioned_as, &fast_path_hash).await?;
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
