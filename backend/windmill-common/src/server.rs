use serde::{Deserialize, Serialize};

use crate::{error, global_settings::SMTP_SETTING, DB};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Smtp {
    pub host: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub port: u16,
    pub from: String,
    pub tls_implicit: Option<bool>,
    pub disable_tls: Option<bool>,
    pub clicktracking_off: Option<bool>,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct SmtpConfigOpt {
    pub smtp_host: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_from: Option<String>,
    pub smtp_tls_implicit: Option<bool>,
    pub smtp_disable_tls: Option<bool>,
    pub smtp_clicktracking_off: Option<bool>,
}

pub async fn load_smtp_config(db: &DB) -> error::Result<Option<Smtp>> {
    let value = crate::global_settings::load_value_from_global_settings(db, SMTP_SETTING).await?;
    Ok(parse_smtp_config(value))
}

/// The half of [`load_smtp_config`] after the read, so a batched settings pass can parse a
/// value it already fetched.
pub fn parse_smtp_config(value: Option<serde_json::Value>) -> Option<Smtp> {
    let config: SmtpConfigOpt = value
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
            tls_implicit: config.smtp_tls_implicit,
            disable_tls: config.smtp_disable_tls,
            port: config.smtp_port.unwrap_or(587),
            from: config
                .smtp_from
                .unwrap_or_else(|| "noreply@getwindmill.com".to_string()),
            clicktracking_off: config.smtp_clicktracking_off,
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
                    .and_then(|p| p.parse().ok()),
                disable_tls: std::env::var("SMTP_DISABLE_TLS")
                    .ok()
                    .and_then(|p| p.parse().ok()),
                port: std::env::var("SMTP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(587),
                from: std::env::var("SMTP_FROM")
                    .unwrap_or_else(|_| "noreply@getwindmill.com".to_string()),
                clicktracking_off: std::env::var("SMTP_CLICKTRACKING_OFF")
                    .ok()
                    .and_then(|p| p.parse().ok()),
            })
        } else {
            None
        },
    );
    if smtp.is_none() {
        tracing::warn!("SMTP not configured");
    }

    smtp
}

impl Default for SmtpConfigOpt {
    fn default() -> Self {
        Self {
            smtp_from: None,
            smtp_host: None,
            smtp_password: None,
            smtp_port: None,
            smtp_tls_implicit: None,
            smtp_username: None,
            smtp_disable_tls: None,
            smtp_clicktracking_off: None,
        }
    }
}
