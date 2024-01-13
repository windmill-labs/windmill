use pg_embed::pg_enums::PgAuthMethod;
use pg_embed::pg_fetch::PgFetchSettings;
use pg_embed::postgres::{PgEmbed, PgSettings};
use std::path::PathBuf;
use std::time::Duration;

pub async fn start() -> anyhow::Result<(String, PgEmbed)> {
    let pg_settings = PgSettings {
        database_dir: PathBuf::from("/tmp/db"),
        port: 6543,
        user: "postgres".to_string(),
        password: "password".to_string(),
        auth_method: PgAuthMethod::Plain,
        persistent: false,
        timeout: Some(Duration::from_secs(15)),
        migration_dir: None,
    };

    let fetch_settings = PgFetchSettings {
        version: pg_embed::pg_fetch::PostgresVersion("15.3.0"),

        ..Default::default()
    };

    tracing::info!(
        "Fetch settings: {:?} {:?}",
        fetch_settings.operating_system,
        fetch_settings.architecture
    );

    let mut pg = PgEmbed::new(pg_settings, fetch_settings).await?;

    pg.setup().await.expect("pg setup");

    pg.start_db().await.expect("pg start db");

    //TODO: re-enable this to make it work
    // if !pg.database_exists("windmill").await.expect("db exists") {
    //     pg.create_database("windmill")
    //         .await
    //         .expect("pg create database");
    // }

    let uri = pg.full_db_uri("windmill");
    Ok((uri, pg))
}
