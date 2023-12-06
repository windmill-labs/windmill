use serde::{Deserialize, Serialize};

use crate::{error, DB};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Smtp {
    pub host: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub port: u16,
    pub from: String,
    pub tls_implicit: bool,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct ServerConfigOpt {
    pub smtp_host: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_from: Option<String>,
    pub smtp_tls_implicit: Option<bool>,
    pub timeout_wait_result: Option<u64>,
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

    let config_smtp = if let (Some(host), username, password) =
        (config.smtp_host, config.smtp_username, config.smtp_password)
    {
        Some(Smtp {
            host,
            username,
            password,
            tls_implicit: config.smtp_tls_implicit.unwrap_or(false),
            port: config.smtp_port.unwrap_or(587),
            from: config
                .smtp_from
                .unwrap_or_else(|| "noreply@getwindmill.com".to_string()),
        })
    } else {
        None
    };
    let smtp = config_smtp.or(
        if let (Some(host), username, password) = (
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

    Ok(ServerConfig {
        smtp,
        timeout_wait_result: config
            .timeout_wait_result
            .ok_or(
                std::env::var("TIMEOUT_WAIT_RESULT")
                    .ok()
                    .and_then(|x| x.parse::<u64>().ok()),
            )
            .unwrap_or(600),
    })
}

impl Default for ServerConfigOpt {
    fn default() -> Self {
        Self {
            smtp_from: None,
            smtp_host: None,
            smtp_password: None,
            smtp_port: None,
            smtp_tls_implicit: None,
            smtp_username: None,
            timeout_wait_result: Default::default(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct ServerConfig {
    pub smtp: Option<Smtp>,
    pub timeout_wait_result: u64,
}
