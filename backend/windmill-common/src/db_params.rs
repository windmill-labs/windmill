use anyhow::Result;
use sqlx::postgres::PgConnectOptions;
use std::str::FromStr;

/// Parsed database connection parameters, shared across DB auth providers (IAM RDS, Entra ID, etc.)
#[derive(Debug, Clone)]
pub struct DatabaseParams {
    pub hostname: String,
    pub port: u64,
    pub username: String,
    pub database: String,
}

/// Extract database connection parameters from a PostgreSQL URL
pub fn extract_database_params(database_url: &str) -> Result<DatabaseParams> {
    let url = url::Url::parse(database_url)
        .map_err(|e| anyhow::anyhow!("Failed to parse database URL: {}", e))?;

    let hostname = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Database URL missing hostname"))?
        .to_string();

    let port = url.port().unwrap_or(5432) as u64;

    let username = if url.username().is_empty() {
        return Err(anyhow::anyhow!("Database URL missing username"));
    } else {
        urlencoding::decode(url.username())?.to_string()
    };

    let database = url
        .path()
        .trim_start_matches('/')
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Database URL missing database name"))?
        .to_string();

    Ok(DatabaseParams {
        hostname,
        port,
        username,
        database: urlencoding::decode(&database)?.to_string(),
    })
}

/// Connection options for a URL whose password an auth provider mints itself.
///
/// Providers must override the password on these rather than assembling options from
/// `DatabaseParams`: the URL's query parameters, `sslmode` and `sslrootcert` above all,
/// live nowhere else, and options built without them drop the TLS policy the operator set.
pub fn base_connect_options(database_url: &str) -> Result<PgConnectOptions> {
    PgConnectOptions::from_str(database_url)
        .map_err(|e| anyhow::anyhow!("Failed to parse database URL: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgSslMode;

    #[test]
    fn base_connect_options_keeps_query_parameters() {
        let opts = base_connect_options(
            "postgres://wm_user:iamrds@db.example.com:5433/windmill?sslmode=verify-full&application_name=windmill",
        )
        .unwrap();

        assert!(matches!(opts.get_ssl_mode(), PgSslMode::VerifyFull));
        assert_eq!(opts.get_application_name(), Some("windmill"));
        assert_eq!(opts.get_host(), "db.example.com");
        assert_eq!(opts.get_port(), 5433);
        assert_eq!(opts.get_username(), "wm_user");
        assert_eq!(opts.get_database(), Some("windmill"));
    }
}
