//! Rendering a `profiles.yml` from a Windmill warehouse resource, and the
//! adapter package each warehouse needs.
//!
//! Only the fields dbt actually reads are emitted; anything the resource
//! carries for other runtimes is ignored rather than guessed at. Credentials
//! are written into the job dir, which is torn down with the job.

use serde_json::Value;
use windmill_common::error::{self, Error};

/// The dbt adapter a Windmill resource type maps to (decision 9). The resource
/// type name is the authority — connection details are never sniffed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbtAdapter {
    Postgres,
    Snowflake,
    Bigquery,
    Databricks,
}

impl DbtAdapter {
    pub fn from_resource_type(rt: &str) -> Option<Self> {
        match rt {
            "postgresql" | "postgres" => Some(DbtAdapter::Postgres),
            "snowflake" | "snowflake_oauth" => Some(DbtAdapter::Snowflake),
            "bigquery" | "gcp_service_account" => Some(DbtAdapter::Bigquery),
            "databricks" => Some(DbtAdapter::Databricks),
            _ => None,
        }
    }

    /// Which dbt driver a resource needs, from the fields it carries. This
    /// picks the adapter only — asset identity is always the resource path,
    /// never anything read here. `profile.type` overrides it.
    pub fn infer_from_resource(v: &Value) -> Option<Self> {
        let has = |k: &str| v.get(k).is_some_and(|x| !x.is_null());
        if has("account_identifier") || has("warehouse") {
            Some(DbtAdapter::Snowflake)
        } else if has("http_path") {
            Some(DbtAdapter::Databricks)
        } else if has("project_id") && has("client_email") {
            Some(DbtAdapter::Bigquery)
        } else if has("dbname") && has("host") {
            Some(DbtAdapter::Postgres)
        } else {
            None
        }
    }

    /// dbt's own `type:` key in `profiles.yml`.
    pub fn dbt_type(&self) -> &'static str {
        match self {
            DbtAdapter::Postgres => "postgres",
            DbtAdapter::Snowflake => "snowflake",
            DbtAdapter::Bigquery => "bigquery",
            DbtAdapter::Databricks => "databricks",
        }
    }

    /// The pip package that provides the adapter for the bundled dbt-core 1.x
    /// engine. The Rust engines ship their adapters in the binary.
    pub fn pip_package(&self) -> &'static str {
        match self {
            DbtAdapter::Postgres => "dbt-postgres",
            DbtAdapter::Snowflake => "dbt-snowflake",
            DbtAdapter::Bigquery => "dbt-bigquery",
            DbtAdapter::Databricks => "dbt-databricks",
        }
    }
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
pub struct RenderedProfile {
    pub yaml: String,
    pub schema: Option<String>,
    pub database: Option<String>,
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
) -> error::Result<RenderedProfile> {
    let mut out: Vec<(String, String)> = vec![("type".into(), adapter.dbt_type().into())];
    let mut schema = schema_override.map(|x| x.to_string());
    let database;

    match adapter {
        DbtAdapter::Postgres => {
            let host = s(resource, "host")
                .ok_or_else(|| Error::BadRequest("postgres resource has no `host`".to_string()))?;
            let dbname = s(resource, "dbname").ok_or_else(|| {
                Error::BadRequest("postgres resource has no `dbname`".to_string())
            })?;
            out.push(("host".into(), host));
            out.push((
                "port".into(),
                n(resource, "port").unwrap_or(5432).to_string(),
            ));
            out.push(("dbname".into(), dbname.clone()));
            database = Some(dbname);
            if let Some(u) = s(resource, "user") {
                out.push(("user".into(), u));
            }
            if let Some(p) = s(resource, "password") {
                out.push(("password".into(), p));
            }
            if let Some(m) = s(resource, "sslmode") {
                out.push(("sslmode".into(), m));
            }
            schema = schema
                .or_else(|| s(resource, "schema"))
                .or(Some("public".into()));
        }
        DbtAdapter::Snowflake => {
            let account = s(resource, "account_identifier")
                .or_else(|| s(resource, "account"))
                .ok_or_else(|| {
                    Error::BadRequest("snowflake resource has no `account_identifier`".to_string())
                })?;
            out.push(("account".into(), account));
            if let Some(u) = s(resource, "username").or_else(|| s(resource, "user")) {
                out.push(("user".into(), u));
            }
            // Key-pair auth is Windmill's snowflake resource shape; password is
            // accepted too for resources that carry one.
            if let Some(k) = s(resource, "private_key") {
                out.push(("private_key".into(), k));
            } else if let Some(p) = s(resource, "password") {
                out.push(("password".into(), p));
            }
            for k in ["warehouse", "role"] {
                if let Some(v) = s(resource, k) {
                    out.push((k.into(), v));
                }
            }
            database = s(resource, "database");
            if let Some(d) = database.clone() {
                out.push(("database".into(), d));
            }
            schema = schema.or_else(|| s(resource, "schema"));
        }
        DbtAdapter::Bigquery => {
            // Windmill's bigquery resource is the raw service-account JSON, so
            // hand dbt the same document via `method: service-account-json`
            // rather than re-deriving individual fields.
            out.push(("method".into(), "service-account-json".into()));
            let project = s(resource, "project_id").ok_or_else(|| {
                Error::BadRequest("bigquery resource has no `project_id`".to_string())
            })?;
            out.push(("project".into(), project.clone()));
            database = Some(project);
            schema = schema.or_else(|| s(resource, "dataset"));
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
                out.push((k.into(), v));
            }
            database = s(resource, "catalog");
            if let Some(c) = database.clone() {
                out.push(("catalog".into(), c));
            }
            schema = schema.or_else(|| s(resource, "schema"));
        }
    }

    if let Some(sc) = schema.clone() {
        out.push(("schema".into(), sc));
    }
    if let Some(t) = threads {
        out.push(("threads".into(), t.to_string()));
    }

    let mut yaml = format!("{profile_name}:\n  target: {target}\n  outputs:\n    {target}:\n");
    for (k, v) in &out {
        yaml.push_str(&format!("      {k}: {}\n", yaml_scalar(v)));
    }
    // The service-account document is a nested mapping, not a scalar.
    if adapter == DbtAdapter::Bigquery {
        yaml.push_str("      keyfile_json:\n");
        let obj = resource
            .as_object()
            .ok_or_else(|| Error::BadRequest("bigquery resource is not an object".to_string()))?;
        for (k, v) in obj {
            if let Some(v) = v.as_str() {
                yaml.push_str(&format!("        {k}: {}\n", yaml_scalar(v)));
            }
        }
    }

    Ok(RenderedProfile { yaml, schema, database })
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
        let p = render_profile(DbtAdapter::Postgres, &r, "wm", "prod", Some(8), None).unwrap();
        assert!(p.yaml.contains("wm:\n  target: prod\n"));
        assert!(p.yaml.contains("      type: \"postgres\"\n"));
        assert!(p.yaml.contains("      port: \"5433\"\n"));
        assert!(p.yaml.contains("      threads: \"8\"\n"));
        assert_eq!(p.database.as_deref(), Some("warehouse"));
        assert_eq!(p.schema.as_deref(), Some("public"));
    }

    // A credential is attacker-influenced text pasted into a resource. Left
    // unquoted, a `\n` or a `"` in it would close the scalar and let the rest
    // of the value be read as further profile keys (e.g. a different `host`),
    // silently redirecting the run at another warehouse.
    #[test]
    fn credentials_cannot_break_out_of_their_scalar() {
        let r = json!({"host": "h", "dbname": "d",
                       "password": "p\"\nhost: evil.example.com\n#"});
        let p = render_profile(DbtAdapter::Postgres, &r, "wm", "dev", None, None).unwrap();
        assert!(p
            .yaml
            .contains(r#"password: "p\"\nhost: evil.example.com\n#""#));
        assert_eq!(p.yaml.matches("host:").count(), 1);
        let parsed: serde_yml::Value = serde_yml::from_str(&p.yaml).unwrap();
        assert_eq!(
            parsed["wm"]["outputs"]["dev"]["host"].as_str(),
            Some("h"),
            "the injected host must not have won"
        );
    }

    #[test]
    fn descriptor_schema_overrides_the_resource() {
        let r = json!({"host": "h", "dbname": "d", "schema": "from_resource"});
        let p = render_profile(
            DbtAdapter::Postgres,
            &r,
            "wm",
            "dev",
            None,
            Some("from_descriptor"),
        )
        .unwrap();
        assert_eq!(p.schema.as_deref(), Some("from_descriptor"));
    }
}
