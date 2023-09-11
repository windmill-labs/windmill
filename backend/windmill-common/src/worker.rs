lazy_static::lazy_static! {
    pub static ref WORKER_GROUP: String = std::env::var("WORKER_GROUP").unwrap_or_else(|_| "default".to_string());
}
