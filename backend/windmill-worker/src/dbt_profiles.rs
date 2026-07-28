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

/// The dbt adapter a Windmill resource type maps to (decision 9). The resource
/// type name is the authority — connection details are never sniffed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbtAdapter {
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

impl DbtAdapter {
    pub fn from_resource_type(rt: &str) -> Option<Self> {
        match rt {
            "postgresql" | "postgres" => Some(DbtAdapter::Postgres),
            "redshift" => Some(DbtAdapter::Redshift),
            "mysql" => Some(DbtAdapter::Mysql),
            "duckdb" => Some(DbtAdapter::Duckdb),
            "clickhouse" => Some(DbtAdapter::Clickhouse),
            "snowflake" | "snowflake_oauth" => Some(DbtAdapter::Snowflake),
            "bigquery" | "gcp_service_account" => Some(DbtAdapter::Bigquery),
            "databricks" => Some(DbtAdapter::Databricks),
            "salesforce" => Some(DbtAdapter::Salesforce),
            "ms_sql_server" | "mssql" | "sqlserver" | "fabric" => Some(DbtAdapter::Mssql),
            "oracledb" | "oracle" => Some(DbtAdapter::OracleDB),
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
            DbtAdapter::Postgres => &AdapterSpec::PG,
            DbtAdapter::Redshift => &AdapterSpec {
                name: "redshift",
                dbt_type: "redshift",
                pip_package: "dbt-redshift",
                default_port: 5439,
                ..AdapterSpec::PG
            },
            DbtAdapter::Mysql => &AdapterSpec {
                name: "mysql",
                dbt_type: "mysql",
                pip_package: "dbt-mysql",
                default_port: 3306,
                database_key: "schema",
                ..AdapterSpec::PG
            },
            DbtAdapter::Duckdb => &AdapterSpec {
                name: "duckdb",
                dbt_type: "duckdb",
                pip_package: "dbt-duckdb",
                ..AdapterSpec::PG
            },
            DbtAdapter::Clickhouse => &AdapterSpec {
                name: "clickhouse",
                dbt_type: "clickhouse",
                pip_package: "dbt-clickhouse",
                ..AdapterSpec::PG
            },
            DbtAdapter::Snowflake => &AdapterSpec {
                name: "snowflake",
                dbt_type: "snowflake",
                pip_package: "dbt-snowflake",
                ..AdapterSpec::PG
            },
            DbtAdapter::Bigquery => &AdapterSpec {
                name: "bigquery",
                dbt_type: "bigquery",
                pip_package: "dbt-bigquery",
                ..AdapterSpec::PG
            },
            DbtAdapter::Databricks => &AdapterSpec {
                name: "databricks",
                dbt_type: "databricks",
                pip_package: "dbt-databricks",
                ..AdapterSpec::PG
            },
            // No dbt-core 1.x package exists for it; Fusion has it built in, and
            // `provision_core_1x` refuses it by name rather than asking uv to
            // install `""`. Pinned by
            // `every_adapter_either_names_a_package_or_is_fusion_only`.
            DbtAdapter::Salesforce => &AdapterSpec {
                name: "salesforce",
                dbt_type: "salesforce",
                pip_package: "",
                ..AdapterSpec::PG
            },
            DbtAdapter::Mssql => &AdapterSpec {
                name: "mssql",
                dbt_type: "sqlserver",
                pip_package: "dbt-sqlserver",
                requires_enterprise: true,
                display_name: Some("Microsoft SQL server"),
                ..AdapterSpec::PG
            },
            DbtAdapter::OracleDB => &AdapterSpec {
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
    /// there is: a second would be the thing that goes stale.
    pub const ALL: &'static [DbtAdapter] = &[
        DbtAdapter::Postgres,
        DbtAdapter::Redshift,
        DbtAdapter::Mysql,
        DbtAdapter::Duckdb,
        DbtAdapter::Clickhouse,
        DbtAdapter::Snowflake,
        DbtAdapter::Bigquery,
        DbtAdapter::Databricks,
        DbtAdapter::Salesforce,
        DbtAdapter::Mssql,
        DbtAdapter::OracleDB,
    ];

    /// Which dbt driver a resource needs, from the fields it carries — a
    /// fallback for when the descriptor omits `profile.type`. It picks the
    /// adapter only; asset identity is always the resource path, never anything
    /// read here.
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
            Some(DbtAdapter::Snowflake)
        } else if has("http_path") {
            Some(DbtAdapter::Databricks)
        } else if has("project_id") && has("client_email") {
            Some(DbtAdapter::Bigquery)
        } else if has("dbname") && has("host") && (has("sslmode") || has("root_certificate_pem")) {
            Some(DbtAdapter::Postgres)
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
pub fn ensure_adapter_licensed(adapter: DbtAdapter) -> error::Result<()> {
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

fn n(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(|x| x.as_i64())
}

/// The rendered `profiles.yml` body plus the `(schema, database)` the target
/// resolves to. The caller needs those two to spell the `table://` asset paths
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
    adapter: DbtAdapter,
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
    let mut out: Vec<(String, ProfileValue)> = vec![("type".into(), quoted(adapter.dbt_type()))];
    let mut schema = schema_override.map(|x| x.to_string());
    let database;

    match adapter {
        // Redshift and MySQL take the same host/port/user/password/database
        // shape as Postgres in both dbt and Windmill's resource types, so one
        // arm renders all three; only the default port and the database key
        // differ.
        DbtAdapter::Postgres | DbtAdapter::Redshift | DbtAdapter::Mysql => {
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
                ProfileValue::Number(n(resource, "port").unwrap_or(adapter.default_port())),
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
                // caller can spell `table://` paths with it.
                DbtAdapter::Mysql => Some(dbname.clone()),
                _ => schema
                    .or_else(|| s(resource, "schema"))
                    .or(Some("public".into())),
            };
        }
        // Not in decision 9's adapter mappings, and their Windmill resources do
        // not carry what dbt needs: an `oracledb` resource is
        // `{user, password, database}` with no host/protocol/service, and
        // dbt-sqlserver requires an ODBC `driver` the images do not install.
        // Rendering a profile from them would produce one that cannot connect,
        // so route them to the project's own `profiles.yml` — the path that
        // exists precisely so an unmodified project runs as-is.
        DbtAdapter::Duckdb
        | DbtAdapter::Clickhouse
        | DbtAdapter::Salesforce
        | DbtAdapter::Mssql
        | DbtAdapter::OracleDB => {
            return Err(Error::BadRequest(format!(
                "the `{}` adapter has no Windmill resource mapping; point \
                 `profile.profiles_yml` at the project's own profiles.yml instead",
                adapter.dbt_type()
            )));
        }
        DbtAdapter::Snowflake => {
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
        DbtAdapter::Bigquery => {
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
        DbtAdapter::Databricks => {
            for (k, rk) in [
                ("host", "host"),
                ("http_path", "http_path"),
                ("token", "token"),
            ] {
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
    if adapter != DbtAdapter::Mysql {
        if let Some(sc) = schema.clone() {
            // dbt-bigquery spells it `dataset`; every other adapter says
            // `schema`. Emitting `schema` there produces a profile dbt rejects.
            let key = match adapter {
                DbtAdapter::Bigquery => "dataset",
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
    let mut yaml =
        format!("{qp}:\n  target: {qt}\n  outputs:\n    {qt}:\n");
    for (k, v) in &out {
        yaml.push_str(&format!("      {k}: {}\n", v.render()));
    }
    // The service-account document is a nested mapping, not a scalar.
    if adapter == DbtAdapter::Bigquery {
        yaml.push_str("      keyfile_json:\n");
        let obj = resource
            .as_object()
            .ok_or_else(|| Error::BadRequest("bigquery resource is not an object".to_string()))?;
        for (k, v) in obj {
            if let Some(v) = v.as_str() {
                yaml.push_str(&format!(
                    "        {}: {}\n",
                    yaml_scalar(k),
                    yaml_scalar(v)
                ));
            }
        }
    }

    Ok(RenderedProfile {
        yaml,
        schema,
        database,
        root_certificate_pem: matches!(adapter, DbtAdapter::Postgres)
            .then(|| s(resource, "root_certificate_pem"))
            .flatten(),
    })
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
            DbtAdapter::Postgres,
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
        assert!(p.yaml.contains("\"wm\":\n  target: \"prod\"\n"), "{}", p.yaml);
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
            DbtAdapter::Redshift,
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

    // A credential is attacker-influenced text pasted into a resource. Left
    // unquoted, a `\n` or a `"` in it would close the scalar and let the rest
    // of the value be read as further profile keys (e.g. a different `host`),
    // silently redirecting the run at another warehouse.
    // dbt-mysql has no database/schema split: one `schema` key is the database.
    // Emitting the generic one too yields a profile with two `schema` keys.
    // A SQL Server resource has the same host/dbname shape as a Postgres one.
    // Guessing Postgres for it would point dbt-postgres at port 1433 instead of
    // producing the licensing error, so an ambiguous resource must decline.
    // dbt rejects a BigQuery target with no dataset, and a service-account JSON
    // carries none — so the descriptor has to supply it, and saying which field
    // is missing beats a downstream dbt error.
    // `snowflake_oauth` maps to the Snowflake adapter, but its credential is a
    // token, which dbt only honors with `authenticator: oauth`. Forwarding
    // neither renders a profile with no credential at all.
    // A resource's private CA is the only way a `verify-full` connection can
    // succeed, and `root_certificate_pem` is also what identifies the resource
    // as Postgres — forwarding one and dropping the other is incoherent.
    #[test]
    fn postgres_forwards_its_root_certificate() {
        let r = json!({"host": "h", "dbname": "d", "sslmode": "verify-full",
                       "root_certificate_pem": "-----BEGIN CERTIFICATE-----\nx\n"});
        let p = render_profile(
            DbtAdapter::Postgres,
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
        assert!(
            p.yaml.contains(&format!(
                "      sslrootcert: \"/tmp/p/{ROOT_CERT_FILENAME}\"\n"
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
            DbtAdapter::Postgres,
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

    #[test]
    fn snowflake_oauth_renders_its_token() {
        let r = json!({"account_identifier": "acc", "username": "u", "token": "tok",
                       "database": "db", "warehouse": "wh"});
        let p = render_profile(
            DbtAdapter::Snowflake,
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

    #[test]
    fn bigquery_requires_a_dataset() {
        let r = json!({"project_id": "p", "client_email": "e", "private_key": "k"});
        let err = render_profile(
            DbtAdapter::Bigquery,
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
            DbtAdapter::Bigquery,
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

    #[test]
    fn ambiguous_host_resources_decline_rather_than_guess() {
        let mssql = json!({"host": "h", "dbname": "d", "user": "u", "password": "p"});
        assert_eq!(DbtAdapter::infer_from_resource(&mssql), None);
        let pg = json!({"host": "h", "dbname": "d", "sslmode": "require"});
        assert_eq!(
            DbtAdapter::infer_from_resource(&pg),
            Some(DbtAdapter::Postgres)
        );
        let sf = json!({"account_identifier": "acc", "database": "d"});
        assert_eq!(
            DbtAdapter::infer_from_resource(&sf),
            Some(DbtAdapter::Snowflake)
        );
    }

    #[test]
    fn mysql_emits_exactly_one_schema_key() {
        let r = json!({"host": "h", "dbname": "sales", "user": "u"});
        let p = render_profile(
            DbtAdapter::Mysql,
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
            DbtAdapter::Postgres,
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
        assert!(rendered.yaml.contains("\"prod # hidden\":"), "{}", rendered.yaml);
        assert!(rendered.yaml.contains("\\n  evil: yes"), "{}", rendered.yaml);
        let v: serde_yml::Value = serde_yml::from_str(&rendered.yaml).expect("valid yaml");
        let profile = v.get("prod # hidden").expect("profile is one key");
        assert!(profile.get("evil").is_none());
        assert!(v.get("evil").is_none());
    }

    #[test]
    fn credentials_cannot_break_out_of_their_scalar() {
        let r = json!({"host": "h", "dbname": "d",
                       "password": "p\"\nhost: evil.example.com\n#"});
        let p = render_profile(
            DbtAdapter::Postgres,
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

    // The dbt adapter gate mirrors the two native script languages still behind
    // an enterprise license, and it is a RUNTIME check because one dbt executor
    // serves every adapter. A refactor that reached for a bare
    // `LICENSE_KEY_VALID` would silently let CE through, since the OSS variant
    // initializes it to `true`.
    #[test]
    fn only_mssql_and_oracle_are_enterprise_gated() {
        for a in [
            DbtAdapter::Postgres,
            DbtAdapter::Redshift,
            DbtAdapter::Mysql,
            DbtAdapter::Duckdb,
            DbtAdapter::Clickhouse,
            DbtAdapter::Snowflake,
            DbtAdapter::Bigquery,
            DbtAdapter::Databricks,
            DbtAdapter::Salesforce,
        ] {
            assert!(!a.requires_enterprise(), "{a:?}");
            assert!(ensure_adapter_licensed(a).is_ok(), "{a:?}");
        }
        for (a, name) in [
            (DbtAdapter::Mssql, "Microsoft SQL server"),
            (DbtAdapter::OracleDB, "Oracle DB"),
        ] {
            assert!(a.requires_enterprise(), "{a:?}");
            match ensure_adapter_licensed(a) {
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
