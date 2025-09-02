/*
 * Author: Claude
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};
use sqlx::{Pool, Postgres};
use tracing::{debug, error, info, warn};
use crate::variables::ListableVariable;

/// Cache TTL for variables and resources (60 seconds)
const CACHE_TTL: Duration = Duration::from_secs(60);

/// Cache entry with expiration
#[derive(Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub expires_at: Instant,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            expires_at: Instant::now() + CACHE_TTL,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

lazy_static::lazy_static! {
    /// Cache for individual variable values: key = "workspace_id:path"
    pub static ref VARIABLE_CACHE: Cache<String, CacheEntry<ListableVariable>> = Cache::new(1000);
    
    /// Cache for resource values: key = "workspace_id:path"
    pub static ref RESOURCE_CACHE: Cache<String, CacheEntry<Value>> = Cache::new(1000);
    
    /// Cache for variable lists: key = "workspace_id:query_hash"
    pub static ref VARIABLE_LIST_CACHE: Cache<String, CacheEntry<Vec<ListableVariable>>> = Cache::new(100);
    
    /// Cache for resource lists: key = "workspace_id:query_hash"  
    pub static ref RESOURCE_LIST_CACHE: Cache<String, CacheEntry<Vec<serde_json::Value>>> = Cache::new(100);
    
    /// Broadcast channel for invalidation notifications
    pub static ref INVALIDATION_SENDER: RwLock<Option<broadcast::Sender<InvalidationMessage>>> = RwLock::new(None);
}

/// Invalidation message types
#[derive(Debug, Clone)]
pub enum InvalidationMessage {
    Variable { workspace_id: String, path: String },
    Resource { workspace_id: String, path: String },
    AllVariables { workspace_id: String },
    AllResources { workspace_id: String },
}

/// Initialize the PostgreSQL LISTEN/NOTIFY system
pub async fn initialize_cache_invalidation(db: &Pool<Postgres>) -> Result<(), crate::error::Error> {
    // Create broadcast channel for invalidation
    let (tx, _rx) = broadcast::channel(1000);
    {
        let mut sender = INVALIDATION_SENDER.write().await;
        *sender = Some(tx.clone());
    }

    // Spawn listener task
    let db_clone = db.clone();
    tokio::spawn(async move {
        if let Err(e) = listen_for_invalidations(db_clone, tx).await {
            error!("Cache invalidation listener failed: {}", e);
        }
    });

    // Create database triggers for cache invalidation
    create_database_triggers(db).await?;
    
    info!("Variable/resource cache invalidation system initialized");
    Ok(())
}

/// Create PostgreSQL triggers for cache invalidation
async fn create_database_triggers(db: &Pool<Postgres>) -> Result<(), crate::error::Error> {
    let mut conn = db.acquire().await?;
    
    // Create notification function
    sqlx::query(r#"
        CREATE OR REPLACE FUNCTION notify_variable_resource_change()
        RETURNS TRIGGER AS $$
        BEGIN
            IF TG_TABLE_NAME = 'variable' THEN
                PERFORM pg_notify('variable_changed', 
                    json_build_object(
                        'workspace_id', COALESCE(NEW.workspace_id, OLD.workspace_id),
                        'path', COALESCE(NEW.path, OLD.path),
                        'operation', TG_OP
                    )::text
                );
            ELSIF TG_TABLE_NAME = 'resource' THEN
                PERFORM pg_notify('resource_changed',
                    json_build_object(
                        'workspace_id', COALESCE(NEW.workspace_id, OLD.workspace_id),
                        'path', COALESCE(NEW.path, OLD.path),
                        'operation', TG_OP
                    )::text
                );
            END IF;
            RETURN COALESCE(NEW, OLD);
        END;
        $$ LANGUAGE plpgsql;
    "#)
    .execute(&mut *conn)
    .await?;

    // Create triggers for variable table
    sqlx::query(r#"
        DROP TRIGGER IF EXISTS variable_cache_invalidate ON variable;
        CREATE TRIGGER variable_cache_invalidate
            AFTER INSERT OR UPDATE OR DELETE ON variable
            FOR EACH ROW EXECUTE FUNCTION notify_variable_resource_change();
    "#)
    .execute(&mut *conn)
    .await?;

    // Create triggers for resource table
    sqlx::query(r#"
        DROP TRIGGER IF EXISTS resource_cache_invalidate ON resource;
        CREATE TRIGGER resource_cache_invalidate
            AFTER INSERT OR UPDATE OR DELETE ON resource
            FOR EACH ROW EXECUTE FUNCTION notify_variable_resource_change();
    "#)
    .execute(&mut *conn)
    .await?;

    debug!("Database triggers for cache invalidation created");
    Ok(())
}

/// Listen for PostgreSQL NOTIFY messages and handle cache invalidation
async fn listen_for_invalidations(
    db: Pool<Postgres>,
    tx: broadcast::Sender<InvalidationMessage>,
) -> Result<(), crate::error::Error> {
    let mut listener = sqlx::postgres::PgListener::connect_with(&db).await?;
    
    listener.listen("variable_changed").await?;
    listener.listen("resource_changed").await?;
    
    debug!("Started listening for cache invalidation notifications");

    loop {
        let notification = listener.recv().await?;
        
        match notification.channel() {
            "variable_changed" => {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(notification.payload()) {
                    if let (Some(workspace_id), Some(path)) = 
                        (payload.get("workspace_id").and_then(|v| v.as_str()),
                         payload.get("path").and_then(|v| v.as_str())) {
                        
                        let msg = InvalidationMessage::Variable {
                            workspace_id: workspace_id.to_string(),
                            path: path.to_string(),
                        };
                        
                        invalidate_variable_cache(&workspace_id, &path);
                        
                        if let Err(e) = tx.send(msg) {
                            warn!("Failed to broadcast variable invalidation: {}", e);
                        }
                    }
                }
            }
            "resource_changed" => {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(notification.payload()) {
                    if let (Some(workspace_id), Some(path)) = 
                        (payload.get("workspace_id").and_then(|v| v.as_str()),
                         payload.get("path").and_then(|v| v.as_str())) {
                        
                        let msg = InvalidationMessage::Resource {
                            workspace_id: workspace_id.to_string(),
                            path: path.to_string(),
                        };
                        
                        invalidate_resource_cache(&workspace_id, &path);
                        
                        if let Err(e) = tx.send(msg) {
                            warn!("Failed to broadcast resource invalidation: {}", e);
                        }
                    }
                }
            }
            _ => {
                warn!("Received unknown notification channel: {}", notification.channel());
            }
        }
    }
}

/// Generate cache key for variables and resources
pub fn cache_key(workspace_id: &str, path: &str) -> String {
    format!("{}:{}", workspace_id, path)
}

/// Generate cache key for list queries
pub fn list_cache_key(workspace_id: &str, query_hash: &str) -> String {
    format!("{}:list:{}", workspace_id, query_hash)
}

/// Get cached variable if available and not expired
pub fn get_cached_variable(workspace_id: &str, path: &str) -> Option<ListableVariable> {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            VARIABLE_CACHE.remove(&key);
            None
        } else {
            debug!("Cache hit for variable {}", key);
            Some(entry.data.clone())
        }
    })
}

/// Cache variable data
pub fn cache_variable(workspace_id: &str, path: &str, variable: ListableVariable) {
    let key = cache_key(workspace_id, path);
    let entry = CacheEntry::new(variable);
    VARIABLE_CACHE.insert(key.clone(), entry);
    debug!("Cached variable {}", key);
}

/// Get cached resource if available and not expired
pub fn get_cached_resource(workspace_id: &str, path: &str) -> Option<Value> {
    let key = cache_key(workspace_id, path);
    RESOURCE_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            RESOURCE_CACHE.remove(&key);
            None
        } else {
            debug!("Cache hit for resource {}", key);
            Some(entry.data.clone())
        }
    })
}

/// Cache resource data
pub fn cache_resource(workspace_id: &str, path: &str, resource: Value) {
    let key = cache_key(workspace_id, path);
    let entry = CacheEntry::new(resource);
    RESOURCE_CACHE.insert(key.clone(), entry);
    debug!("Cached resource {}", key);
}

/// Invalidate specific variable from cache
pub fn invalidate_variable_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.remove(&key);
    
    // Also invalidate related list caches
    let list_prefix = format!("{}:list:", workspace_id);
    let keys_to_remove: Vec<String> = VARIABLE_LIST_CACHE
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| k.starts_with(&list_prefix))
        .collect();
    
    for key in keys_to_remove {
        VARIABLE_LIST_CACHE.remove(&key);
    }
    
    debug!("Invalidated variable cache for {}", key);
}

/// Invalidate specific resource from cache
pub fn invalidate_resource_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    RESOURCE_CACHE.remove(&key);
    
    // Also invalidate related list caches
    let list_prefix = format!("{}:list:", workspace_id);
    let keys_to_remove: Vec<String> = RESOURCE_LIST_CACHE
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| k.starts_with(&list_prefix))
        .collect();
    
    for key in keys_to_remove {
        RESOURCE_LIST_CACHE.remove(&key);
    }
    
    debug!("Invalidated resource cache for {}", key);
}

/// Subscribe to invalidation notifications
pub async fn subscribe_to_invalidations() -> Option<broadcast::Receiver<InvalidationMessage>> {
    let sender = INVALIDATION_SENDER.read().await;
    sender.as_ref().map(|s| s.subscribe())
}

/// Clear all caches (for testing/debugging)
pub fn clear_all_caches() {
    VARIABLE_CACHE.clear();
    RESOURCE_CACHE.clear();
    VARIABLE_LIST_CACHE.clear();
    RESOURCE_LIST_CACHE.clear();
    debug!("All variable/resource caches cleared");
}

/// Health check for cache system
pub async fn cache_health_check() -> bool {
    // Check if invalidation sender is initialized
    let sender_ok = {
        let sender = INVALIDATION_SENDER.read().await;
        sender.is_some()
    };

    if !sender_ok {
        warn!("Variable/resource cache invalidation system not initialized");
        return false;
    }

    true
}