use windmill_common::initial_connection;

#[tokio::main]
async fn main() {
    let db = initial_connection().await.unwrap();
    let result = sqlx::query_scalar!("SELECT version()").fetch_one(&db).await;
    // fetch all from scripts table
    let scripts = sqlx::query!("SELECT path FROM script").fetch_all(&db).await;
    match result {
        Ok(version) => match version {
            Some(version) => println!("PostgreSQL version: {}", version),
            None => println!("PostgreSQL version: unknown"),
        },
        Err(e) => println!("Failed to get PostgreSQL version: {}", e),
    }
    match scripts {
        Ok(scripts) => println!("Scripts: {:?}", scripts),
        Err(e) => println!("Failed to get scripts: {}", e),
    }
}
