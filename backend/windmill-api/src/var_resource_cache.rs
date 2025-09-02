/*
 * Author: Claude
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use sqlx::{Pool, Postgres};
use tracing::{debug, error, info, warn};
use windmill_common::variables::ListableVariable;

/// Cache TTL for variables and resources (60 seconds)
const CACHE_TTL_SECS: u64 = 60;

/// Cache entry with timestamp and value (following raw script cache pattern)
#[derive(Clone, Debug)]
pub struct CacheEntry<T> {
    pub timestamp: u64,
    pub value: T,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            value,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.timestamp + CACHE_TTL_SECS
    }
}

/// Invalidation message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvalidationMessage {
    Variable { workspace_id: String, path: String },
    Resource { workspace_id: String, path: String },
}

lazy_static::lazy_static! {
    /// Cache for individual variable values: key = "workspace_id:path"
    static ref VARIABLE_CACHE: Cache<String, CacheEntry<ListableVariable>> = Cache::new(1000);
    
    /// Cache for resource values: key = "workspace_id:path"  
    static ref RESOURCE_CACHE: Cache<String, CacheEntry<Value>> = Cache::new(1000);
    
    /// Broadcast channel for invalidation notifications
    static ref INVALIDATION_SENDER: tokio::sync::RwLock<Option<broadcast::Sender<InvalidationMessage>>> = 
        tokio::sync::RwLock::new(None);
}

/// Initialize the PostgreSQL LISTEN/NOTIFY system
pub async fn initialize_cache_invalidation(db: &Pool<Postgres>) -> Result<(), sqlx::Error> {
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
    
    info!("Variable/resource cache invalidation system initialized");
    Ok(())
}

/// Listen for PostgreSQL NOTIFY messages and handle cache invalidation
async fn listen_for_invalidations(
    db: Pool<Postgres>,
    tx: broadcast::Sender<InvalidationMessage>,
) -> Result<(), sqlx::Error> {
    let mut listener = sqlx::postgres::PgListener::connect_with(&db).await?;
    
    listener.listen("var_cache_invalidation").await?;
    listener.listen("resource_cache_invalidation").await?;
    
    debug!("Started listening for cache invalidation notifications");

    loop {
        let notification = listener.recv().await?;
        
        match notification.channel() {
            "var_cache_invalidation" => {
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
            "resource_cache_invalidation" => {
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

/// Get cached variable if available and not expired  
pub fn get_cached_variable(workspace_id: &str, path: &str) -> Option<ListableVariable> {
    let key = cache_key(workspace_id, path);
    VARIABLE_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            VARIABLE_CACHE.remove(&key);
            None
        } else {
            debug!("Cache hit for variable {}", key);
            Some(entry.value.clone())
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
            Some(entry.value.clone())
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
    debug!("Invalidated variable cache for {}", key);
}

/// Invalidate specific resource from cache
pub fn invalidate_resource_cache(workspace_id: &str, path: &str) {
    let key = cache_key(workspace_id, path);
    RESOURCE_CACHE.remove(&key);
    debug!("Invalidated resource cache for {}", key);
}

/// Subscribe to invalidation notifications
#[allow(dead_code)]
pub async fn subscribe_to_invalidations() -> Option<broadcast::Receiver<InvalidationMessage>> {
    let sender = INVALIDATION_SENDER.read().await;
    sender.as_ref().map(|s| s.subscribe())
}

/// Clear all caches (for testing/debugging)
#[allow(dead_code)]
pub fn clear_all_caches() {
    VARIABLE_CACHE.clear();
    RESOURCE_CACHE.clear();
    debug!("All variable/resource caches cleared");
}

/// Health check for cache system
#[allow(dead_code)]
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