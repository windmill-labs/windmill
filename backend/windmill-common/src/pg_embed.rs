/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::error::Error;
use postgresql_embedded::{PostgreSQL, Settings};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages an embedded PostgreSQL instance
pub struct EmbeddedPostgres {
    postgresql: Arc<RwLock<PostgreSQL>>,
    database_url: String,
}

impl EmbeddedPostgres {
    /// Initialize and start an embedded PostgreSQL instance
    pub async fn new() -> Result<Self, Error> {
        tracing::info!("Initializing embedded PostgreSQL instance...");

        // Configure the embedded PostgreSQL settings
        let mut settings = Settings::default();

        // Use environment variables if provided for customization
        if let Ok(data_dir) = std::env::var("PG_EMBED_DATA_DIR") {
            settings.installation_dir = data_dir.into();
        }

        if let Ok(port) = std::env::var("PG_EMBED_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                settings.port = port_num;
            }
        }

        let mut postgresql = PostgreSQL::new(settings);

        // Setup the PostgreSQL instance
        postgresql
            .setup()
            .await
            .map_err(|e| {
                let err_msg = format!("Failed to setup embedded PostgreSQL: {}", e);
                if err_msg.contains("libxml2") || err_msg.contains("shared libraries") {
                    Error::InternalErr(format!(
                        "{}\n\n\
                        System dependencies are required for embedded PostgreSQL.\n\
                        On Arch Linux, install: sudo pacman -S libxml2 icu openssl\n\
                        On Ubuntu/Debian: sudo apt-get install libxml2 libicu-dev libssl-dev\n\
                        On RHEL/Fedora: sudo dnf install libxml2 libicu openssl-libs\n\n\
                        Alternatively, set DATABASE_URL to use an external PostgreSQL instance.",
                        err_msg
                    ))
                } else {
                    Error::InternalErr(err_msg)
                }
            })?;

        tracing::info!("Starting embedded PostgreSQL...");

        // Start the PostgreSQL instance
        postgresql
            .start()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to start embedded PostgreSQL: {}", e)))?;

        tracing::info!("Embedded PostgreSQL started successfully");

        // Get the database settings
        let settings = postgresql.settings();

        // Create the windmill database
        let database_name = std::env::var("PG_EMBED_DATABASE")
            .unwrap_or_else(|_| "windmill".to_string());

        tracing::info!("Creating database: {}", database_name);

        postgresql
            .create_database(&database_name)
            .await
            .map_err(|e| {
                Error::InternalErr(format!("Failed to create database '{}': {}", database_name, e))
            })?;

        // Build the connection string
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            settings.username,
            settings.password,
            settings.host,
            settings.port,
            database_name
        );

        tracing::info!("Embedded PostgreSQL ready at: postgres://{}:{}@{}:{}/{}",
            settings.username,
            "***",  // Don't log password
            settings.host,
            settings.port,
            database_name
        );

        Ok(Self {
            postgresql: Arc::new(RwLock::new(postgresql)),
            database_url,
        })
    }

    /// Get the database connection URL
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Stop the embedded PostgreSQL instance
    pub async fn stop(&self) -> Result<(), Error> {
        tracing::info!("Stopping embedded PostgreSQL...");

        let pg = self.postgresql.write().await;
        pg.stop()
            .await
            .map_err(|e| Error::InternalErr(format!("Failed to stop embedded PostgreSQL: {}", e)))?;

        tracing::info!("Embedded PostgreSQL stopped");
        Ok(())
    }
}

/// Initialize an embedded PostgreSQL instance and set the DATABASE_URL environment variable
pub async fn init_embedded_postgres() -> Result<EmbeddedPostgres, Error> {
    let embedded_pg = EmbeddedPostgres::new().await?;

    // Set the DATABASE_URL environment variable so that the rest of the application
    // can use it transparently
    unsafe {
        std::env::set_var("DATABASE_URL", embedded_pg.database_url());
    }

    tracing::info!("DATABASE_URL set to embedded PostgreSQL instance");

    Ok(embedded_pg)
}
