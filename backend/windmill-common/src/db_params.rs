use anyhow::Result;

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
