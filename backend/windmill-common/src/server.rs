use serde::{Deserialize, Serialize};

use crate::{error, DB};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Smtp {
    pub host: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub from: String,
    pub tls_implicit: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ServerConfigOpt {
    pub smtp: Option<Smtp>,
}

pub async fn load_server_config(db: &DB) -> error::Result<ServerConfig> {
    let config: ServerConfigOpt =
        sqlx::query_scalar!("SELECT config FROM config WHERE name = 'server'",)
            .fetch_optional(db)
            .await?
            .flatten()
            .map(|x| serde_json::from_value(x).ok())
            .flatten()
            .unwrap_or_default();

    let smtp = config.smtp.or(
        if let (Some(host), Some(username), Some(password)) = (
            std::env::var("SMTP_HOST").ok(),
            std::env::var("SMTP_USERNAME").ok(),
            std::env::var("SMTP_PASSWORD").ok(),
        ) {
            Some(Smtp {
                host,
                username,
                password,
                tls_implicit: std::env::var("SMTP_TLS_IMPLICIT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(false),
                port: std::env::var("SMTP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(587),
                from: std::env::var("SMTP_FROM")
                    .unwrap_or_else(|_| "noreply@getwindmill.com".to_string()),
            })
        } else {
            None
        },
    );
    if smtp.is_none() {
        tracing::warn!("SMTP not configured");
    }

    Ok(ServerConfig { smtp })
}

impl Default for ServerConfigOpt {
    fn default() -> Self {
        Self { smtp: Default::default() }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct ServerConfig {
    pub smtp: Option<Smtp>,
}
