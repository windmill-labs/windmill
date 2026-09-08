//! Rendering a `profiles.yml` from a Windmill warehouse resource, and the
//! adapter package each warehouse needs.
//!
//! Only the fields dbt actually reads are emitted; anything the resource
//! carries for other runtimes is ignored rather than guessed at. Credentials
//! are written into the job dir, which is torn down with the job.

use serde_json::Value;
use windmill_common::error::{self, Error};

/// Written beside `profiles.yml`, and named absolutely in `sslrootcert`: dbt
/// runs with the project as its working directory and hands the path to the
/// driver unchanged.
pub const ROOT_CERT_FILENAME: &str = "server-ca.pem";

/// The per-adapter facts, so each adapter states them together and a new one
/// cannot inherit another's by omission. `PG` is the base every arm spreads
/// from: most adapters differ from Postgres only in their name and package.
struct AdapterSpec {
    /// As a user would write it, for error messages.
    name: &'static str,
    /// dbt's own `type:` key in `profiles.yml`.
    dbt_type: &'static str,
    /// The dbt-core 1.x pip package; empty when the adapter is Fusion-only.
    pip_package: &'static str,
    default_port: i64,
    database_key: &'static str,
    requires_enterprise: bool,
    /// Overrides `dbt_type` in the licensing error, where the product's own
    /// spelling reads better than dbt's driver name.
    display_name: Option<&'static str>,
}

impl AdapterSpec {
    const PG: AdapterSpec = AdapterSpec {
        name: "postgres",
        dbt_type: "postgres",
        pip_package: "dbt-postgres",
        default_port: 5432,
        database_key: "dbname",
        requires_enterprise: false,
        display_name: None,
    };
}

/// The adapters Windmill has facts about (decision 9): a field mapping, a pip package, the
/// license gate. NOT the adapters dbt projects may use — `DbtAdapter` carries those by name.
/// The picker offers what reaches one (`WAREHOUSE_RESOURCE_TYPES` in
/// `frontend/.../workspaceSettings/DbtSettings.svelte`), so a mapping change belongs there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownAdapter {
    Postgres,
    Redshift,
    Mysql,
    Duckdb,
    Clickhouse,
    Snowflake,
    Bigquery,
    Databricks,
    Salesforce,
    Mssql,
    OracleDB,
}

impl KnownAdapter {
    /// dbt's OWN `type:` spelling — a separate vocabulary from the resource types
    /// below, deliberately. `fabric` is why: Windmill's `fabric` RESOURCE is a SQL
    /// Server one, while dbt's `fabric` ADAPTER is its own `dbt-fabric`. Absent
    /// here, it falls through to the open path and gets what it asked for.
    pub fn from_dbt_type(t: &str) -> Option<Self> {
        match t {
            "postgres" | "postgresql" => Some(KnownAdapter::Postgres),
            "redshift" => Some(KnownAdapter::Redshift),
            "mysql" => Some(KnownAdapter::Mysql),
            "duckdb" => Some(KnownAdapter::Duckdb),
            "clickhouse" => Some(KnownAdapter::Clickhouse),
            "snowflake" => Some(KnownAdapter::Snowflake),
            "bigquery" => Some(KnownAdapter::Bigquery),
            "databricks" => Some(KnownAdapter::Databricks),
            "salesforce" => Some(KnownAdapter::Salesforce),
            "sqlserver" | "mssql" => Some(KnownAdapter::Mssql),
            "oracle" => Some(KnownAdapter::OracleDB),
            _ => None,
        }
    }

    pub fn from_resource_type(rt: &str) -> Option<Self> {
        match rt {
            "postgresql" | "postgres" => Some(KnownAdapter::Postgres),
            "redshift" => Some(KnownAdapter::Redshift),
            "mysql" => Some(KnownAdapter::Mysql),
            "duckdb" => Some(KnownAdapter::Duckdb),
            "clickhouse" => Some(KnownAdapter::Clickhouse),
            "snowflake" | "snowflake_oauth" => Some(KnownAdapter::Snowflake),
            "bigquery" | "gcp_service_account" => Some(KnownAdapter::Bigquery),
            "databricks" => Some(KnownAdapter::Databricks),
            "salesforce" => Some(KnownAdapter::Salesforce),
            "ms_sql_server" | "mssql" | "sqlserver" | "fabric" => Some(KnownAdapter::Mssql),
            "oracledb" | "oracle" => Some(KnownAdapter::OracleDB),
            _ => None,
        }
    }

    /// Everything that differs per adapter, in one place.
    ///
    /// Adding a warehouse is one arm here, and the match is exhaustive so the
    /// compiler demands it. Spread across a method each, two of them carried a
    /// `_ =>` default, so a new adapter silently inherited port 5432 and
    /// `dbname` instead of failing to build.
    fn spec(&self) -> &'static AdapterSpec {
        match self {
            KnownAdapter::Postgres => &AdapterSpec::PG,
            KnownAdapter::Redshift => &AdapterSpec {
                name: "redshift",
                dbt_type: "redshift",
                pip_package: "dbt-redshift",
                default_port: 5439,
                ..AdapterSpec::PG
            },
            KnownAdapter::Mysql => &AdapterSpec {
                name: "mysql",
                dbt_type: "mysql",
                pip_package: "dbt-mysql",
                default_port: 3306,
                database_key: "schema",
                ..AdapterSpec::PG
            },
            KnownAdapter::Duckdb => &AdapterSpec {
                name: "duckdb",
                dbt_type: "duckdb",
                pip_package: "dbt-duckdb",
                ..AdapterSpec::PG
            },
            KnownAdapter::Clickhouse => &AdapterSpec {
                name: "clickhouse",
                dbt_type: "clickhouse",
                pip_package: "dbt-clickhouse",
                ..AdapterSpec::PG
            },
            KnownAdapter::Snowflake => &AdapterSpec {
                name: "snowflake",
                dbt_type: "snowflake",
                pip_package: "dbt-snowflake",
                ..AdapterSpec::PG
            },
            KnownAdapter::Bigquery => &AdapterSpec {
                name: "bigquery",
                dbt_type: "bigquery",
                pip_package: "dbt-bigquery",
                ..AdapterSpec::PG
            },
            KnownAdapter::Databricks => &AdapterSpec {
                name: "databricks",
                dbt_type: "databricks",
                pip_package: "dbt-databricks",
                ..AdapterSpec::PG
            },
            // No dbt-core 1.x package exists for it; Fusion has it built in, and
            // `provision_core_1x` refuses it by name rather than asking uv to
            // install `""`. Pinned by
            // `every_adapter_either_names_a_package_or_is_fusion_only`.
            KnownAdapter::Salesforce => &AdapterSpec {
                name: "salesforce",
                dbt_type: "salesforce",
                pip_package: "",
                ..AdapterSpec::PG
            },
            KnownAdapter::Mssql => &AdapterSpec {
                name: "mssql",
                dbt_type: "sqlserver",
                pip_package: "dbt-sqlserver",
                requires_enterprise: true,
                display_name: Some("Microsoft SQL server"),
                ..AdapterSpec::PG
            },
            KnownAdapter::OracleDB => &AdapterSpec {
                name: "oracle",
                dbt_type: "oracle",
                pip_package: "dbt-oracle",
                requires_enterprise: true,
                display_name: Some("Oracle DB"),
                ..AdapterSpec::PG
            },
        }
    }

    /// Every adapter, for the tests that must cover all of them. The one list
    /// there is: a second would be the thing that goes stale. Test-only, so a
    /// release build does not carry a table nothing reads.
    #[cfg(test)]
    pub const ALL: &'static [KnownAdapter] = &[
        KnownAdapter::Postgres,
        KnownAdapter::Redshift,
        KnownAdapter::Mysql,
        KnownAdapter::Duckdb,
        KnownAdapter::Clickhouse,
        KnownAdapter::Snowflake,
        KnownAdapter::Bigquery,
        KnownAdapter::Databricks,
        KnownAdapter::Salesforce,
        KnownAdapter::Mssql,
        KnownAdapter::OracleDB,
    ];

    /// Which dbt driver a resource needs, from the fields it carries — a
    /// fallback for when the descriptor omits `profile.type`. It picks the
    /// adapter only; asset identity is always the workspace warehouse's name,
    /// never anything read here.
    ///
    /// Deliberately conservative about the host/port shapes. Windmill's
    /// `ms_sql_server` and `oracledb` resources carry the same `host` +
    /// `dbname`/`database` fields Postgres does, so a shape check cannot tell
    /// them apart — and guessing Postgres for a SQL Server resource would
    /// connect dbt-postgres to port 1433 instead of producing the licensing
    /// error this design exists to give. So a bare host/database resource is
    /// only Postgres when it carries something Postgres-specific, and is
    /// otherwise `None`, which asks the user for `profile.type`.
    pub fn infer_from_resource(v: &Value) -> Option<Self> {
        let has = |k: &str| v.get(k).is_some_and(|x| !x.is_null());
        if has("account_identifier") || has("warehouse") {
            Some(KnownAdapter::Snowflake)
        } else if has("http_path") {
            Some(KnownAdapter::Databricks)
        } else if has("project_id") && has("client_email") {
            Some(KnownAdapter::Bigquery)
        } else if has("dbname") && has("host") && (has("sslmode") || has("root_certificate_pem")) {
            Some(KnownAdapter::Postgres)
        } else {
            None
        }
    }

    /// dbt's own `type:` key in `profiles.yml`.
    pub fn dbt_type(&self) -> &'static str {
        self.spec().dbt_type
    }

    /// The adapter's name as a user would write it, for error messages.
    pub fn name(&self) -> &'static str {
        self.spec().name
    }

    /// The pip package providing this adapter for the dbt-core 1.x engine,
    /// whose venv is provisioned on first use. The Rust engines ship their
    /// adapters in the binary and never consult this. Empty means no such
    /// package exists.
    pub fn pip_package(&self) -> &'static str {
        self.spec().pip_package
    }

    /// Only meaningful for the host/port adapters rendered from a resource.
    fn default_port(&self) -> i64 {
        self.spec().default_port
    }

    /// dbt spells the database differently per adapter.
    fn database_key(&self) -> &'static str {
        self.spec().database_key
    }

    /// The keys a target block spells its database and schema with, mirroring
    /// what `render_profile` emits below.
    ///
    /// Read back when the PROJECT owns its `profiles.yml`. Windmill did not
    /// write that file, but it still needs the target's database to spell
    /// `dbt://` paths the way a rendered profile does: without it every relation
    /// qualifies as `<db>.<schema>` while a workspace-warehouse project spells
    /// the same table plainly, and the two never share a node.
    pub fn target_identity_keys(&self) -> (&'static str, &'static str) {
        match self {
            KnownAdapter::Bigquery => ("project", "dataset"),
            KnownAdapter::Databricks => ("catalog", "schema"),
            // `database_key` is what Windmill's own resource spells it, and only
            // the adapters `render_profile` translates have one. The rest reach
            // dbt through a block they wrote themselves, which spells it the way
            // dbt does.
            KnownAdapter::Postgres | KnownAdapter::Redshift | KnownAdapter::Mysql => {
                (self.database_key(), "schema")
            }
            _ => ("database", "schema"),
        }
    }

    /// Whether this adapter needs an enterprise license.
    ///
    /// The boundary is not invented for dbt: it mirrors the two native script
    /// languages that are still enterprise-gated. Every other warehouse dbt can
    /// reach has a CE `ScriptLang`, so gating its dbt adapter would be stricter
    /// than running the same query natively.
    pub fn requires_enterprise(&self) -> bool {
        self.spec().requires_enterprise
    }

    /// The name used in the licensing error, matching how the native languages
    /// name themselves.
    fn display_name(&self) -> &'static str {
        let spec = self.spec();
        spec.display_name.unwrap_or(spec.dbt_type)
    }
}

/// The adapter a profile connects with: dbt's own `type:`, plus whatever Windmill knows.
/// Open by construction — a `dbt_profile` carries a block Windmill never has to understand,
/// so an unknown adapter is still rendered, licensed and identified. INSTALLING one is a
/// separate question, gated by `ensure_adapter_installable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbtAdapter {
    known: Option<KnownAdapter>,
    /// dbt's own `type:` spelling, lowercased. Also the pip package's suffix.
    name: String,
}

impl DbtAdapter {
    /// An adapter as dbt spells it, known or not. The name reaches a pip requirement
    /// and a venv path, so it is confined to what an adapter name can be rather than
    /// escaped at each use: a leading `-` is a pip flag, a `/` or `..` a path segment.
    pub fn from_dbt_type(t: &str) -> error::Result<Self> {
        let name = t.trim().to_ascii_lowercase();
        let shaped = name.len() <= 40
            && name.starts_with(|c: char| c.is_ascii_alphanumeric())
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !shaped {
            return Err(Error::BadRequest(format!(
                "`{t}` is not a dbt adapter name: dbt spells one with letters, digits, `_` and \
                 `-`, as in `postgres` or `trino`"
            )));
        }
        // Normalised to the adapter's own dbt spelling when one resolves, so two
        // names for one adapter are one value: `PartialEq` covers `name` too, and
        // `profile.type: mssql` over a `sqlserver` target would otherwise be
        // rejected by a message naming the same adapter on both sides.
        let known = KnownAdapter::from_dbt_type(&name);
        let name = known.map_or(name, |k| k.dbt_type().to_string());
        Ok(Self { known, name })
    }

    /// The adapter a `dbt_profile` states, from the `type` of the block its value is.
    /// Called only for that resource TYPE, never sniffed: Windmill's bigquery resource
    /// is a service-account JSON and says `type: service_account`.
    pub fn stated_by_dbt_profile(v: &Value) -> error::Result<Self> {
        let stated = v.get("type").and_then(|t| t.as_str()).ok_or_else(|| {
            Error::BadRequest(
                "a `dbt_profile` resource names its adapter in `type`, as a `profiles.yml` output \
                 does; this one has none"
                    .to_string(),
            )
        })?;
        Self::from_dbt_type(stated)
    }

    /// The adapter's facts, when it is one Windmill has any.
    pub fn known(&self) -> Option<KnownAdapter> {
        self.known
    }

    /// dbt's own `type:` key in `profiles.yml`.
    pub fn dbt_type(&self) -> &str {
        self.known.map_or(self.name.as_str(), |k| k.dbt_type())
    }

    /// The adapter's name as a user would write it, for error messages.
    pub fn name(&self) -> &str {
        self.known.map_or(self.name.as_str(), |k| k.name())
    }

    /// The pip package providing this adapter for the dbt-core 1.x engine.
    /// Empty means no such package exists and only Fusion has it.
    pub fn pip_package(&self) -> String {
        match self.known {
            Some(k) => k.pip_package().to_string(),
            None => format!("dbt-{}", self.name),
        }
    }

    /// The keys a target block spells its database and schema with.
    /// An unknown adapter is read with dbt's ordinary pair.
    pub fn target_identity_keys(&self) -> (&'static str, &'static str) {
        self.known
            .map_or(("database", "schema"), |k| k.target_identity_keys())
    }

    /// Whether this adapter needs an enterprise license. Never for an unknown
    /// one: the gate mirrors the two native warehouse languages, and an adapter
    /// with no Windmill runtime behind it is not one of them.
    pub fn requires_enterprise(&self) -> bool {
        self.known.is_some_and(|k| k.requires_enterprise())
    }

    fn display_name(&self) -> &str {
        self.known.map_or(self.name.as_str(), |k| k.display_name())
    }
}

impl From<KnownAdapter> for DbtAdapter {
    fn from(known: KnownAdapter) -> Self {
        Self { known: Some(known), name: known.dbt_type().to_string() }
    }
}

/// Whether this build may use the enterprise-only adapters.
///
/// Two conditions, both required. `LICENSE_KEY_VALID` alone is not enough: the
/// OSS variant of `ee_oss` initializes it to `true`, so reading it on a CE
/// build would wave everything through. The `cfg` establishes it is an
/// enterprise build; the atomic then reports whether that build's key actually
/// verified.
fn enterprise_licensed() -> bool {
    #[cfg(feature = "enterprise")]
    {
        windmill_common::ee_oss::LICENSE_KEY_VALID.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(not(feature = "enterprise"))]
    {
        false
    }
}

/// Reject an enterprise-only adapter on an unlicensed build.
///
/// Unlike the native languages, this cannot be a compile-time gate: there is
/// one dbt executor and the adapter is only known once the profile resolves. So
/// it is checked at both deploy and run — and it says so plainly, rather than
/// letting the run fail later with a connection error the user cannot act on.
pub fn ensure_adapter_licensed(adapter: &DbtAdapter) -> error::Result<()> {
    if adapter.requires_enterprise() && !enterprise_licensed() {
        return Err(Error::BadRequest(format!(
            "{} is only available with an enterprise license",
            adapter.display_name()
        )));
    }
    Ok(())
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|x| x.to_string())
        .filter(|x| !x.is_empty())
}

/// A resource's port, which may arrive as a JSON number or as a string —
/// resources are user-supplied JSON and nothing coerces the field. Reading only
/// `as_i64` sent `"5433"` to the adapter default, which connects to whatever
/// listens there and reports nothing; a value that is present but not a port is
/// refused rather than replaced.
fn port_of(resource: &Value, default: i64) -> error::Result<i64> {
    let bad = |v: &Value| {
        Error::BadRequest(format!(
            "resource `port` must be a number, got `{v}`; correct the resource"
        ))
    };
    match resource.get("port") {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => n.as_i64().ok_or_else(|| bad(&Value::Number(n.clone()))),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(default),
        Some(Value::String(s)) => s
            .trim()
            .parse::<i64>()
            .map_err(|_| bad(&Value::String(s.clone()))),
        Some(other) => Err(bad(other)),
    }
}

/// The rendered `profiles.yml` body plus the `(schema, database)` the target
/// resolves to. The caller needs those two to spell the `dbt://` asset paths
/// of models that do not override them.
#[derive(Debug)]
pub struct RenderedProfile {
    pub yaml: String,
    pub schema: Option<String>,
    pub database: Option<String>,
    /// A private CA the caller must write next to `profiles.yml`, under the
    /// file name `sslrootcert` already points at. Returned rather than written
    /// here so this stays a pure renderer.
    pub root_certificate_pem: Option<String>,
}

/// Render a single-target `profiles.yml` for `profile_name`/`target`.
///
/// `threads` is the descriptor's, defaulted by dbt when absent. `schema` comes
/// from the descriptor when set, else from the resource, else from dbt's own
/// per-adapter default — dbt errors out clearly when it ends up missing, which
/// is a better failure than a Windmill-invented default.
pub fn render_profile(
    adapter: &DbtAdapter,
    resource: &Value,
    profile_name: &str,
    target: &str,
    threads: Option<u32>,
    schema_override: Option<&str>,
    // Where `profiles.yml` (and any root certificate) is written. dbt runs with
    // the PROJECT as its working directory and hands `sslrootcert` to the
    // driver unchanged, so a path relative to the profiles dir would be
    // resolved against the project and never found.
    profiles_dir: &std::path::Path,
) -> error::Result<RenderedProfile> {
    // Past here the resource is one of Windmill's own connection types, which
    // becomes a target only through a field mapping below — so an adapter
    // Windmill has no facts about cannot be rendered from one at all.
    let adapter = adapter.known().ok_or_else(|| {
        Error::BadRequest(format!(
            "no Windmill resource translates into a `{}` target; point the warehouse at a \
             `dbt_profile` resource, whose value is the profiles.yml block itself",
            adapter.dbt_type()
        ))
    })?;

    let mut out: Vec<(String, ProfileValue)> = vec![("type".into(), quoted(adapter.dbt_type()))];
    let mut schema = schema_override.map(|x| x.to_string());
    let database;

    match adapter {
        // Redshift and MySQL take the same host/port/user/password/database
        // shape as Postgres in both dbt and Windmill's resource types, so one
        // arm renders all three; only the default port and the database key
        // differ.
        KnownAdapter::Postgres | KnownAdapter::Redshift | KnownAdapter::Mysql => {
            let host = s(resource, "host").ok_or_else(|| {
                Error::BadRequest(format!("{} resource has no `host`", adapter.dbt_type()))
            })?;
            let dbname = s(resource, "dbname")
                .or_else(|| s(resource, "database"))
                .or_else(|| s(resource, "service_name"))
                .ok_or_else(|| {
                    Error::BadRequest(format!(
                        "{} resource has no `dbname`/`database`",
                        adapter.dbt_type()
                    ))
                })?;
            out.push(("host".into(), quoted(&host)));
            out.push((
                "port".into(),
                // dbt validates `port` against a JSON schema that demands an
                // integer; a quoted scalar is rejected outright.
                ProfileValue::Number(port_of(resource, adapter.default_port())?),
            ));
            out.push((adapter.database_key().into(), quoted(&dbname)));
            database = Some(dbname.clone());
            if let Some(u) = s(resource, "user") {
                out.push(("user".into(), quoted(&u)));
            }
            if let Some(p) = s(resource, "password") {
                out.push(("password".into(), quoted(&p)));
            }
            if let Some(m) = s(resource, "sslmode") {
                out.push(("sslmode".into(), quoted(&m)));
            }
            // Under `verify-ca`/`verify-full` a resource's private CA is the
            // only way the connection can succeed; without it libpq looks for a
            // default certificate that is not there, and the same field is what
            // identifies the resource as Postgres in the first place.
            if s(resource, "root_certificate_pem").is_some() {
                out.push((
                    "sslrootcert".into(),
                    quoted(&profiles_dir.join(ROOT_CERT_FILENAME).to_string_lossy()),
                ));
            }
            schema = match adapter {
                // Already emitted as the database key; reported back so the
                // caller can spell `dbt://` paths with it.
                KnownAdapter::Mysql => Some(dbname.clone()),
                _ => schema
                    .or_else(|| s(resource, "schema"))
                    .or(Some("public".into())),
            };
        }
        // Their Windmill resources do not carry what dbt needs — an `oracledb`
        // resource has no host/service, dbt-sqlserver needs an ODBC `driver` the
        // images lack — so a rendered profile could not connect. They are reached
        // through a target written for them instead.
        KnownAdapter::Duckdb
        | KnownAdapter::Clickhouse
        | KnownAdapter::Salesforce
        | KnownAdapter::Mssql
        | KnownAdapter::OracleDB => {
            return Err(Error::BadRequest(format!(
                "a `{}` resource carries nothing dbt can connect with; point the warehouse at a \
                 `dbt_profile` resource, whose value is the profiles.yml block itself, or \
                 `profile.profiles_yml` at the project's own profiles.yml",
                adapter.dbt_type()
            )));
        }
        KnownAdapter::Snowflake => {
            let account = s(resource, "account_identifier")
                .or_else(|| s(resource, "account"))
                .ok_or_else(|| {
                    Error::BadRequest("snowflake resource has no `account_identifier`".to_string())
                })?;
            out.push(("account".into(), quoted(&account)));
            if let Some(u) = s(resource, "username").or_else(|| s(resource, "user")) {
                out.push(("user".into(), quoted(&u)));
            }
            // Key-pair is Windmill's own snowflake resource shape; the
            // `snowflake_oauth` type carries a token instead, which dbt only
            // accepts alongside `authenticator: oauth` — without both, the
            // profile renders with no credential at all and cannot connect.
            if let Some(t) = s(resource, "token").or_else(|| s(resource, "access_token")) {
                out.push(("authenticator".into(), quoted("oauth")));
                out.push(("token".into(), quoted(&t)));
            } else if let Some(k) = s(resource, "private_key") {
                out.push(("private_key".into(), quoted(&k)));
                if let Some(pp) = s(resource, "private_key_passphrase") {
                    out.push(("private_key_passphrase".into(), quoted(&pp)));
                }
            } else if let Some(p) = s(resource, "password") {
                out.push(("password".into(), quoted(&p)));
            }
            for k in ["warehouse", "role"] {
                if let Some(v) = s(resource, k) {
                    out.push((k.into(), quoted(&v)));
                }
            }
            database = s(resource, "database");
            if let Some(d) = database.clone() {
                out.push(("database".into(), quoted(&d)));
            }
            schema = schema.or_else(|| s(resource, "schema"));
        }
        KnownAdapter::Bigquery => {
            // Windmill's bigquery resource is the raw service-account JSON, so
            // hand dbt the same document via `method: service-account-json`
            // rather than re-deriving individual fields.
            out.push(("method".into(), quoted("service-account-json")));
            let project = s(resource, "project_id").ok_or_else(|| {
                Error::BadRequest("bigquery resource has no `project_id`".to_string())
            })?;
            out.push(("project".into(), quoted(&project)));
            database = Some(project);
            // A service-account JSON has no dataset, so unless the resource was
            // extended with one the descriptor must supply it — dbt rejects a
            // BigQuery target without `dataset`, and failing here names the
            // missing field instead.
            schema = schema
                .or_else(|| s(resource, "dataset"))
                .or_else(|| s(resource, "schema"));
            if schema.is_none() {
                return Err(Error::BadRequest(
                    "a BigQuery target needs a dataset; set `profile.schema` in the descriptor"
                        .to_string(),
                ));
            }
        }
        KnownAdapter::Databricks => {
            // Windmill's databricks resource spells the workspace `workspace_url`, and
            // spells it as a full URL; dbt wants a bare hostname under `host`.
            let host = s(resource, "host")
                .or_else(|| s(resource, "workspace_url").map(|u| bare_host(&u)))
                .ok_or_else(|| {
                    Error::BadRequest(
                        "databricks resource has no `workspace_url`/`host`".to_string(),
                    )
                })?;
            out.push(("host".into(), quoted(&host)));
            for (k, rk) in [("http_path", "http_path"), ("token", "token")] {
                let v = s(resource, rk).ok_or_else(|| {
                    Error::BadRequest(format!("databricks resource has no `{rk}`"))
                })?;
                out.push((k.into(), quoted(&v)));
            }
            database = s(resource, "catalog");
            if let Some(c) = database.clone() {
                out.push(("catalog".into(), quoted(&c)));
            }
            schema = schema.or_else(|| s(resource, "schema"));
        }
    }

    // dbt-mysql has no database/schema distinction: its `schema` key IS the
    // database, already emitted above. Pushing the generic one too would put
    // two `schema` keys in one target — an invalid profile, or one silently
    // pointing at the wrong database.
    if adapter != KnownAdapter::Mysql {
        if let Some(sc) = schema.clone() {
            // dbt-bigquery spells it `dataset`; every other adapter says
            // `schema`. Emitting `schema` there produces a profile dbt rejects.
            let key = match adapter {
                KnownAdapter::Bigquery => "dataset",
                _ => "schema",
            };
            out.push((key.into(), quoted(&sc)));
        }
    }
    if let Some(t) = threads {
        out.push(("threads".into(), ProfileValue::Number(t as i64)));
    }

    // Quoted, keys included. `profile_name` comes from the project's own
    // `dbt_project.yml` and `target` from the descriptor, so both are the
    // author's text: a name like `prod # x` silently truncates the mapping, and
    // a newline in one opens a sibling key of the caller's choosing.
    let (qp, qt) = (yaml_scalar(profile_name), yaml_scalar(target));
    let mut yaml = format!("{qp}:\n  target: {qt}\n  outputs:\n    {qt}:\n");
    for (k, v) in &out {
        yaml.push_str(&format!("      {k}: {}\n", v.render()));
    }
    // The service-account document is a nested mapping, not a scalar.
    if adapter == KnownAdapter::Bigquery {
        yaml.push_str("      keyfile_json:\n");
        let obj = resource
            .as_object()
            .ok_or_else(|| Error::BadRequest("bigquery resource is not an object".to_string()))?;
        for (k, v) in obj {
            if let Some(v) = v.as_str() {
                yaml.push_str(&format!("        {}: {}\n", yaml_scalar(k), yaml_scalar(v)));
            }
        }
    }

    Ok(RenderedProfile {
        yaml,
        schema,
        database,
        root_certificate_pem: matches!(adapter, KnownAdapter::Postgres)
            .then(|| s(resource, "root_certificate_pem"))
            .flatten(),
    })
}

/// Render a `dbt_profile`: its value IS one entry of `profiles.yml`'s `outputs` map, so it
/// is emitted as it stands — nothing lifted out or renamed, which is the point of the type.
/// Only what dbt cannot take literally is handled: the adapter's `type`, a certificate that
/// is a PEM body rather than a path, and the two keys a descriptor may override.
pub fn render_dbt_profile(
    adapter: &DbtAdapter,
    block: &serde_json::Map<String, Value>,
    profile_name: &str,
    target: &str,
    threads: Option<u32>,
    schema_override: Option<&str>,
    profiles_dir: &std::path::Path,
) -> error::Result<RenderedProfile> {
    let (database_key, schema_key) = adapter.target_identity_keys();
    let root_certificate_pem = block
        .get("root_certificate_pem")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());

    let (qp, qt) = (yaml_scalar(profile_name), yaml_scalar(target));
    let mut yaml = format!("{qp}:\n  target: {qt}\n  outputs:\n    {qt}:\n");
    // Keys quoted throughout this block, the block's own and Windmill's alike: a
    // target whose `"host"` is quoted and whose `schema` is not reads as a bug.
    yaml.push_str(&format!(
        "      \"type\": {}\n",
        yaml_scalar(adapter.dbt_type())
    ));
    for (k, v) in block {
        // A null is an optional field the resource form left unset, and dbt
        // validates several keys against a schema that rejects one.
        if k == "type" || k == "root_certificate_pem" || v.is_null() {
            continue;
        }
        // Only when Windmill writes one of its own, which would otherwise be a second
        // `sslrootcert`. A path with no PEM beside it is the block's own trust source —
        // a CA baked into the image or mounted on the worker — and dropping it changes
        // what the connection verifies against.
        if k == "sslrootcert" && root_certificate_pem.is_some() {
            continue;
        }
        if (k == schema_key && schema_override.is_some()) || (k == "threads" && threads.is_some()) {
            continue;
        }
        emit_entry(&mut yaml, 6, k, v);
    }
    if root_certificate_pem.is_some() {
        yaml.push_str(&format!(
            "      \"sslrootcert\": {}\n",
            yaml_scalar(&profiles_dir.join(ROOT_CERT_FILENAME).to_string_lossy())
        ));
    }
    if let Some(sc) = schema_override {
        yaml.push_str(&format!(
            "      {}: {}\n",
            yaml_scalar(schema_key),
            yaml_scalar(sc)
        ));
    }
    if let Some(t) = threads {
        yaml.push_str(&format!("      \"threads\": {t}\n"));
    }

    let str_key = |k: &str| block.get(k).and_then(|v| v.as_str()).map(|v| v.to_string());
    Ok(RenderedProfile {
        yaml,
        schema: schema_override
            .map(|x| x.to_string())
            .or_else(|| str_key(schema_key)),
        database: str_key(database_key),
        root_certificate_pem,
    })
}

/// Emit one target key, nesting as deep as the value goes — an adapter's credential can be
/// a mapping (bigquery's `keyfile_json`) or a list. Keys are quoted like values: one nothing
/// here enumerates is as free-form as a password.
fn emit_entry(out: &mut String, indent: usize, key: &str, v: &Value) {
    out.push_str(&format!("{}{}:", " ".repeat(indent), yaml_scalar(key)));
    emit_value(out, indent, v);
}

/// The value half, after `key:`. An empty collection is emitted INLINE: a block with no
/// children reads back as `null`, so `extensions: []` would reach the adapter as a missing
/// value rather than the empty list dbt was handed.
fn emit_value(out: &mut String, indent: usize, v: &Value) {
    match v {
        Value::Object(m) => {
            // A null is an optional field the resource form left unset, and dbt validates
            // several keys against a schema that rejects one.
            let kept: Vec<_> = m.iter().filter(|(_, v)| !v.is_null()).collect();
            if kept.is_empty() {
                out.push_str(" {}\n");
                return;
            }
            out.push('\n');
            for (k, v) in kept {
                emit_entry(out, indent + 2, k, v);
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str(" []\n");
                return;
            }
            out.push('\n');
            let pad = " ".repeat(indent + 2);
            for item in items {
                out.push_str(&pad);
                out.push('-');
                emit_value(out, indent + 2, item);
            }
        }
        _ => out.push_str(&format!(" {}\n", yaml_value(v))),
    }
}

/// A scalar as dbt reads it: quoted when it is text, bare when it is not.
/// `port`, `threads` and the boolean toggles adapters carry are validated
/// against a JSON schema that rejects the quoted form.
fn yaml_value(v: &Value) -> String {
    match v {
        Value::String(s) => yaml_scalar(s),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => yaml_scalar(&other.to_string()),
    }
}

/// One rendered `profiles.yml` value. Strings are always quoted so credential
/// content cannot inject profile keys; numbers must NOT be, because dbt
/// validates `port` and `threads` against a JSON schema that demands integers
/// and rejects the quoted form outright.
enum ProfileValue {
    Str(String),
    Number(i64),
}

impl ProfileValue {
    fn render(&self) -> String {
        match self {
            ProfileValue::Str(s) => yaml_scalar(s),
            ProfileValue::Number(n) => n.to_string(),
        }
    }
}

fn quoted(v: &str) -> ProfileValue {
    ProfileValue::Str(v.to_string())
}

/// A workspace URL as dbt wants it: the hostname alone. Windmill's databricks
/// resource carries the full deployment URL, while dbt-databricks builds its own
/// URL from `host`, so a scheme left in place produces `https://https://…`.
fn bare_host(url: &str) -> String {
    url.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

/// Emit a YAML scalar that survives any credential content. Always
/// double-quoted with the two characters that can terminate or escape inside a
/// double-quoted scalar escaped, so a password containing `#`, `:`, a newline
/// or a quote cannot break out of its value and inject profile keys.
fn yaml_scalar(v: &str) -> String {
    let escaped = v
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_postgres_target() {
        let r = json!({"host": "db.internal", "port": 5433, "user": "u", "password": "p",
                       "dbname": "warehouse", "sslmode": "require"});
        let p = render_profile(
            &KnownAdapter::Postgres.into(),
            &r,
            "wm",
            "prod",
            Some(8),
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        // Quoted: the profile name and target are the author's text, so they are
        // rendered as scalars rather than as bare keys.
        assert!(
            p.yaml.contains("\"wm\":\n  target: \"prod\"\n"),
            "{}",
            p.yaml
        );
        assert!(p.yaml.contains("      type: \"postgres\"\n"));
        // dbt's profile schema types these as integers, so they must not be
        // quoted like the credential scalars around them.
        assert!(p.yaml.contains("      port: 5433\n"));
        assert!(p.yaml.contains("      threads: 8\n"));
        assert_eq!(p.database.as_deref(), Some("warehouse"));
        assert_eq!(p.schema.as_deref(), Some("public"));
    }

    #[test]
    fn redshift_falls_back_to_its_own_port() {
        // Redshift is rendered from the same host/port shape as Postgres but
        // does not answer on Postgres's port.
        let r = json!({"host": "cluster.redshift.amazonaws.com", "user": "u",
                       "password": "p", "dbname": "warehouse"});
        let p = render_profile(
            &KnownAdapter::Redshift.into(),
            &r,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(p.yaml.contains("      port: 5439\n"), "{}", p.yaml);
    }

    // The point of `dbt_profile`: an adapter with no field mapping still
    // connects, because its value is a block dbt wrote and reaches dbt as it is.
    #[test]
    fn dbt_profile_renders_its_block_verbatim() {
        let r = json!({"type": "clickhouse", "host": "ch.internal", "port": 8123,
                       "secure": true, "user": "u", "password": "p", "schema": "analytics"});
        let p = render_dbt_profile(
            &KnownAdapter::Clickhouse.into(),
            r.as_object().unwrap(),
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(
            p.yaml.contains("      \"type\": \"clickhouse\"\n"),
            "{}",
            p.yaml
        );
        assert!(
            p.yaml.contains("      \"host\": \"ch.internal\"\n"),
            "{}",
            p.yaml
        );
        // dbt types these in its own schema and rejects the quoted form.
        assert!(p.yaml.contains("      \"port\": 8123\n"), "{}", p.yaml);
        assert!(p.yaml.contains("      \"secure\": true\n"), "{}", p.yaml);
        // The identity a `dbt://` path is spelled with, read back from the block.
        assert_eq!(p.schema.as_deref(), Some("analytics"));
    }

    // Two `schema` keys in one target is a profile dbt rejects, or one silently
    // pointing at the schema the descriptor meant to override.
    #[test]
    fn dbt_profile_schema_override_replaces_the_block_key() {
        let r = json!({"type": "clickhouse", "host": "h", "schema": "raw"});
        let p = render_dbt_profile(
            &KnownAdapter::Clickhouse.into(),
            r.as_object().unwrap(),
            "wm",
            "prod",
            None,
            Some("staging"),
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert_eq!(p.yaml.matches("\"schema\":").count(), 1, "{}", p.yaml);
        assert!(p.yaml.contains("\"schema\": \"staging\"\n"), "{}", p.yaml);
        assert_eq!(p.schema.as_deref(), Some("staging"));
    }

    // `fabric` is a distinct dbt adapter AND a Windmill resource type for SQL
    // Server. Resolving dbt's `type:` through the resource table installed
    // dbt-sqlserver for it, under an enterprise gate, and never said Fabric.
    #[test]
    fn a_dbt_type_is_not_a_resource_type() {
        let fabric = DbtAdapter::from_dbt_type("fabric").unwrap();
        assert_eq!(fabric.dbt_type(), "fabric");
        assert_eq!(fabric.pip_package(), "dbt-fabric");
        assert!(!fabric.requires_enterprise());
        // The Windmill resource type keeps mapping where it always did.
        assert_eq!(
            KnownAdapter::from_resource_type("fabric"),
            Some(KnownAdapter::Mssql)
        );
    }

    // `PartialEq` covers the carried name, so two spellings of one adapter must
    // normalise or the descriptor/resource agreement check rejects a valid
    // config with a message naming the same adapter on both sides.
    #[test]
    fn two_spellings_of_one_adapter_are_one_value() {
        for (a, b) in [("postgres", "postgresql"), ("sqlserver", "mssql")] {
            assert_eq!(
                DbtAdapter::from_dbt_type(a).unwrap(),
                DbtAdapter::from_dbt_type(b).unwrap(),
                "{a} vs {b}"
            );
        }
    }

    // An adapter Windmill has no facts about is still an adapter: this is what
    // "whatever dbt supports" rests on.
    #[test]
    fn an_unknown_adapter_is_carried_by_name() {
        let stated = DbtAdapter::stated_by_dbt_profile(&json!({"type": "trino", "host": "h"}))
            .unwrap();
        assert_eq!(stated.dbt_type(), "trino");
        assert_eq!(stated.pip_package(), "dbt-trino");
        assert!(stated.known().is_none());
        // A block with no adapter cannot be rendered, and says which key is missing.
        assert!(DbtAdapter::stated_by_dbt_profile(&json!({"host": "h"})).is_err());
    }

    // A block whose CA is a path on the worker keeps it: Windmill only takes the
    // key over when it has a PEM of its own to point at.
    #[test]
    fn a_path_only_sslrootcert_survives() {
        let r = json!({"type": "postgres", "host": "h", "sslmode": "verify-full",
                       "sslrootcert": "/etc/ssl/certs/warehouse-ca.pem"});
        let p = render_dbt_profile(
            &KnownAdapter::Postgres.into(),
            r.as_object().unwrap(),
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(
            p.yaml
                .contains("\"sslrootcert\": \"/etc/ssl/certs/warehouse-ca.pem\"\n"),
            "{}",
            p.yaml
        );
        // And a PEM in the block still wins, exactly once.
        let r = json!({"type": "postgres", "host": "h", "sslrootcert": "/ignored",
                       "root_certificate_pem": "-----BEGIN CERTIFICATE-----\nx\n"});
        let p = render_dbt_profile(
            &KnownAdapter::Postgres.into(),
            r.as_object().unwrap(),
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert_eq!(p.yaml.matches("sslrootcert").count(), 1, "{}", p.yaml);
        assert!(p.yaml.contains(ROOT_CERT_FILENAME), "{}", p.yaml);
    }

    // dbt hands these to the adapter as it finds them, so a collection must survive the
    // round trip: a block with no children reads back as `null`, not as `[]` or `{}`.
    #[test]
    fn collections_keep_their_type() {
        let r = json!({"type": "duckdb", "extensions": [], "settings": {},
                       "attach": [{"path": "raw.db", "read_only": true}],
                       "matrix": [["a", 1], []], "plugins": ["excel", "json"]});
        let p = render_dbt_profile(
            &DbtAdapter::from_dbt_type("duckdb").unwrap(),
            r.as_object().unwrap(),
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        // Parsed back rather than string-matched: the point is what a YAML reader sees.
        let y: serde_json::Value = serde_yml::from_str(&p.yaml).expect(&p.yaml);
        let t = &y["wm"]["outputs"]["prod"];
        assert_eq!(t["extensions"], json!([]), "{}", p.yaml);
        assert_eq!(t["settings"], json!({}), "{}", p.yaml);
        assert_eq!(t["attach"], json!([{"path": "raw.db", "read_only": true}]), "{}", p.yaml);
        assert_eq!(t["matrix"], json!([["a", 1], []]), "{}", p.yaml);
        assert_eq!(t["plugins"], json!(["excel", "json"]), "{}", p.yaml);
    }

    // An unknown adapter's name becomes `dbt-<name>` in a pip requirement and a
    // venv path, both on the host and outside the jail.
    #[test]
    fn an_adapter_name_cannot_be_a_pip_flag_or_a_path() {
        for bad in [
            "--index-url=http://evil",
            "-e .",
            "../../etc/passwd",
            "dbt postgres",
            "",
        ] {
            assert!(DbtAdapter::from_dbt_type(bad).is_err(), "{bad}");
        }
        assert_eq!(
            DbtAdapter::from_dbt_type("TRINO").unwrap().dbt_type(),
            "trino"
        );
    }

    // Windmill's databricks resource spells the workspace as a full URL under
    // `workspace_url`; dbt-databricks builds its own URL from a bare `host`.
    #[test]
    fn databricks_takes_its_host_from_the_workspace_url() {
        let r = json!({"workspace_url": "https://dbc-a1b2.cloud.databricks.com/",
                       "http_path": "/sql/1.0/warehouses/x", "token": "t"});
        let p = render_profile(
            &KnownAdapter::Databricks.into(),
            &r,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(
            p.yaml
                .contains("      host: \"dbc-a1b2.cloud.databricks.com\"\n"),
            "{}",
            p.yaml
        );
    }

    // A resource's private CA is the only way a `verify-full` connection can
    // succeed, and `root_certificate_pem` is also what identifies the resource as
    // Postgres — forwarding one and dropping the other is incoherent.
    #[test]
    fn postgres_forwards_its_root_certificate() {
        let r = json!({"host": "h", "dbname": "d", "sslmode": "verify-full",
                       "root_certificate_pem": "-----BEGIN CERTIFICATE-----\nx\n"});
        let p = render_profile(
            &KnownAdapter::Postgres.into(),
            &r,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        // Absolute: dbt runs with the PROJECT as its cwd and hands this to the
        // driver unchanged, so a profiles-relative path would never be found.
        let cert_path = std::path::Path::new("/tmp/p").join(ROOT_CERT_FILENAME);
        assert!(
            p.yaml.contains(&format!(
                "      sslrootcert: {}\n",
                yaml_scalar(&cert_path.to_string_lossy())
            )),
            "{}",
            p.yaml
        );
        assert_eq!(
            p.root_certificate_pem.as_deref(),
            Some("-----BEGIN CERTIFICATE-----\nx\n")
        );
        // No CA configured, no dangling sslrootcert pointing at a missing file.
        let plain = json!({"host": "h", "dbname": "d", "sslmode": "require"});
        let p = render_profile(
            &KnownAdapter::Postgres.into(),
            &plain,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(!p.yaml.contains("sslrootcert"));
        assert_eq!(p.root_certificate_pem, None);
    }

    // `snowflake_oauth` maps to the Snowflake adapter, but its credential is a
    // token, which dbt honors only with `authenticator: oauth`. Forwarding neither
    // renders a profile with no credential at all.
    #[test]
    fn snowflake_oauth_renders_its_token() {
        let r = json!({"account_identifier": "acc", "username": "u", "token": "tok",
                       "database": "db", "warehouse": "wh"});
        let p = render_profile(
            &KnownAdapter::Snowflake.into(),
            &r,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(
            p.yaml.contains("      authenticator: \"oauth\"\n"),
            "{}",
            p.yaml
        );
        assert!(p.yaml.contains("      token: \"tok\"\n"));
    }

    // dbt rejects a BigQuery target with no dataset and a service-account JSON
    // carries none, so the descriptor has to supply it — and naming the missing
    // field beats a downstream dbt error.
    #[test]
    fn bigquery_requires_a_dataset() {
        let r = json!({"project_id": "p", "client_email": "e", "private_key": "k"});
        let err = render_profile(
            &KnownAdapter::Bigquery.into(),
            &r,
            "wm",
            "prod",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("profile.schema"), "{err}");
        let p = render_profile(
            &KnownAdapter::Bigquery.into(),
            &r,
            "wm",
            "prod",
            None,
            Some("marts"),
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        // dbt-bigquery's key is `dataset`, not `schema`.
        assert!(p.yaml.contains("      dataset: \"marts\"\n"), "{}", p.yaml);
        assert!(!p.yaml.contains("      schema:"));
        assert_eq!(p.schema.as_deref(), Some("marts"));
    }

    // A SQL Server resource has a Postgres resource's shape, so guessing Postgres
    // points dbt-postgres at port 1433 instead of producing the licensing error.
    #[test]
    fn ambiguous_host_resources_decline_rather_than_guess() {
        let mssql = json!({"host": "h", "dbname": "d", "user": "u", "password": "p"});
        assert_eq!(KnownAdapter::infer_from_resource(&mssql), None);
        let pg = json!({"host": "h", "dbname": "d", "sslmode": "require"});
        assert_eq!(
            KnownAdapter::infer_from_resource(&pg),
            Some(KnownAdapter::Postgres)
        );
        let sf = json!({"account_identifier": "acc", "database": "d"});
        assert_eq!(
            KnownAdapter::infer_from_resource(&sf),
            Some(KnownAdapter::Snowflake)
        );
    }

    // dbt-mysql has no database/schema split: one `schema` key is the database, so
    // emitting the generic one too yields a profile with two.
    #[test]
    fn mysql_emits_exactly_one_schema_key() {
        let r = json!({"host": "h", "dbname": "sales", "user": "u"});
        let p = render_profile(
            &KnownAdapter::Mysql.into(),
            &r,
            "wm",
            "dev",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert_eq!(p.yaml.matches("      schema:").count(), 1);
        assert!(p.yaml.contains("      schema: \"sales\"\n"));
        assert!(p.yaml.contains("      port: 3306\n"));
        assert_eq!(p.schema.as_deref(), Some("sales"));
    }

    // The profile name is the project's own `dbt_project.yml`, the target the
    // descriptor's — both the author's text, and both were mapping keys.
    #[test]
    fn a_profile_name_or_target_cannot_open_a_sibling_key() {
        let rendered = render_profile(
            &KnownAdapter::Postgres.into(),
            &serde_json::json!({"host": "h", "user": "u", "password": "p", "dbname": "d"}),
            "prod # hidden",
            "dev\n  evil: yes",
            None,
            None,
            std::path::Path::new("/tmp"),
        )
        .unwrap();
        // Every occurrence is quoted, so neither the comment nor the newline is
        // structure: the document still has exactly the keys we wrote.
        assert!(
            rendered.yaml.contains("\"prod # hidden\":"),
            "{}",
            rendered.yaml
        );
        assert!(
            rendered.yaml.contains("\\n  evil: yes"),
            "{}",
            rendered.yaml
        );
        let v: serde_yml::Value = serde_yml::from_str(&rendered.yaml).expect("valid yaml");
        let profile = v.get("prod # hidden").expect("profile is one key");
        assert!(profile.get("evil").is_none());
        assert!(v.get("evil").is_none());
    }

    // A credential is attacker-influenced text. Unquoted, a `\n` or `"` closes the
    // scalar and the rest reads as further profile keys — a different `host`,
    // silently redirecting the run at another warehouse.
    #[test]
    fn credentials_cannot_break_out_of_their_scalar() {
        let r = json!({"host": "h", "dbname": "d",
                       "password": "p\"\nhost: evil.example.com\n#"});
        let p = render_profile(
            &KnownAdapter::Postgres.into(),
            &r,
            "wm",
            "dev",
            None,
            None,
            std::path::Path::new("/tmp/p"),
        )
        .unwrap();
        assert!(p
            .yaml
            .contains(r#"password: "p\"\nhost: evil.example.com\n#""#));
        let parsed: serde_yml::Value = serde_yml::from_str(&p.yaml).unwrap();
        assert_eq!(
            parsed["wm"]["outputs"]["dev"]["host"].as_str(),
            Some("h"),
            "the injected host must not have won"
        );
    }

    // A RUNTIME check, because one dbt executor serves every adapter. A refactor
    // reaching for a bare `LICENSE_KEY_VALID` would silently let CE through, since
    // the OSS variant initializes it to `true`.
    #[test]
    fn only_mssql_and_oracle_are_enterprise_gated() {
        for a in [
            KnownAdapter::Postgres,
            KnownAdapter::Redshift,
            KnownAdapter::Mysql,
            KnownAdapter::Duckdb,
            KnownAdapter::Clickhouse,
            KnownAdapter::Snowflake,
            KnownAdapter::Bigquery,
            KnownAdapter::Databricks,
            KnownAdapter::Salesforce,
        ] {
            assert!(!a.requires_enterprise(), "{a:?}");
            assert!(ensure_adapter_licensed(&a.into()).is_ok(), "{a:?}");
        }
        for (a, name) in [
            (KnownAdapter::Mssql, "Microsoft SQL server"),
            (KnownAdapter::OracleDB, "Oracle DB"),
        ] {
            assert!(a.requires_enterprise(), "{a:?}");
            match ensure_adapter_licensed(&a.into()) {
                Ok(()) => assert!(
                    enterprise_licensed(),
                    "{a:?} was accepted without an enterprise license"
                ),
                Err(e) => {
                    assert!(!enterprise_licensed(), "{a:?} rejected despite a license");
                    // Naming the adapter is the point: the user must not be left
                    // with a generic connection failure to diagnose.
                    assert_eq!(
                        e.to_string(),
                        format!("Bad request: {name} is only available with an enterprise license")
                    );
                }
            }
        }
    }
}
