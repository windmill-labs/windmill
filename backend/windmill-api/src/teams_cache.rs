/*
 * Author: Claude
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::teams_ee::{TeamRecord, ChannelRecord};
use quick_cache::sync::Cache;
use std::time::{SystemTime, UNIX_EPOCH};

/// Cache TTL for Teams data (5 minutes)
/// Teams and channel IDs are immutable, so we can cache for longer periods
const CACHE_TTL_SECS: u64 = 300; // 5 minutes

/// Cache entry with timestamp and value
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

lazy_static::lazy_static! {
    /// Cache for team search results: key = "search:{search_term}" or "exact:{team_name}"
    /// Capacity: 500 entries (teams don't change frequently)
    pub static ref TEAMS_CACHE: Cache<String, CacheEntry<Vec<TeamRecord>>> = Cache::new(500);

    /// Cache for channel search results: key = "team:{team_id}:search:{search_term}" or "team:{team_id}:exact:{channel_name}"
    /// Capacity: 1000 entries (more channels than teams)
    pub static ref CHANNELS_CACHE: Cache<String, CacheEntry<Vec<ChannelRecord>>> = Cache::new(1000);
}

/// Generate cache key for teams search
pub fn teams_cache_key(search_term: &str) -> String {
    format!("search:{}", search_term)
}

/// Generate cache key for exact team name lookup
pub fn team_exact_cache_key(team_name: &str) -> String {
    format!("exact:{}", team_name)
}

/// Generate cache key for channels search
pub fn channels_cache_key(team_id: &str, search_term: Option<&str>) -> String {
    match search_term {
        Some(term) => format!("team:{}:search:{}", team_id, term),
        None => format!("team:{}:search:", team_id), // Empty search = list all
    }
}

/// Generate cache key for exact channel name lookup
pub fn channel_exact_cache_key(team_id: &str, channel_name: &str) -> String {
    format!("team:{}:exact:{}", team_id, channel_name)
}

/// Get cached teams if available and not expired
pub fn get_cached_teams(search_term: &str) -> Option<Vec<TeamRecord>> {
    let key = teams_cache_key(search_term);
    TEAMS_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            TEAMS_CACHE.remove(&key);
            None
        } else {
            tracing::info!("Teams cache hit for search: {}", search_term);
            Some(entry.value.clone())
        }
    })
}

/// Cache teams search results
pub fn cache_teams(search_term: &str, teams: Vec<TeamRecord>) {
    let key = teams_cache_key(search_term);
    let entry = CacheEntry::new(teams.clone());
    TEAMS_CACHE.insert(key, entry);
    tracing::info!("Cached {} teams for search: {}", teams.len(), search_term);
}

/// Get cached channels if available and not expired
pub fn get_cached_channels(team_id: &str, search_term: Option<&str>) -> Option<Vec<ChannelRecord>> {
    let key = channels_cache_key(team_id, search_term);
    CHANNELS_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            CHANNELS_CACHE.remove(&key);
            None
        } else {
            let search_display = search_term.unwrap_or("");
            tracing::info!("Channels cache hit for team {} search: {}", team_id, search_display);
            Some(entry.value.clone())
        }
    })
}

/// Cache channels search results
pub fn cache_channels(team_id: &str, search_term: Option<&str>, channels: Vec<ChannelRecord>) {
    let key = channels_cache_key(team_id, search_term);
    let entry = CacheEntry::new(channels.clone());
    CHANNELS_CACHE.insert(key, entry);
    let search_display = search_term.unwrap_or("");
    tracing::info!("Cached {} channels for team {} search: {}", channels.len(), team_id, search_display);
}

/// Get cached team by exact name lookup - returns the team if found
pub fn get_cached_team_by_name(team_name: &str) -> Option<String> {
    let key = team_exact_cache_key(team_name);
    TEAMS_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            TEAMS_CACHE.remove(&key);
            None
        } else {
            tracing::info!("Team exact cache hit for name: {}", team_name);
            // Return the team ID from the first (and only) team in the cached result
            entry.value.first().and_then(|team| team.team_id.clone())
        }
    })
}

/// Cache single team result for exact name lookup
pub fn cache_team_by_name(team_name: &str, team_record: TeamRecord) {
    let key = team_exact_cache_key(team_name);
    let entry = CacheEntry::new(vec![team_record.clone()]);
    TEAMS_CACHE.insert(key, entry);
    tracing::info!("Cached team {} for exact name: {}", team_record.team_id.as_ref().unwrap_or(&"unknown".to_string()), team_name);
}

/// Get cached channel by exact name lookup - returns the channel ID if found
pub fn get_cached_channel_by_name(team_id: &str, channel_name: &str) -> Option<String> {
    let key = channel_exact_cache_key(team_id, channel_name);
    CHANNELS_CACHE.get(&key).and_then(|entry| {
        if entry.is_expired() {
            CHANNELS_CACHE.remove(&key);
            None
        } else {
            tracing::info!("Channel exact cache hit for team {} channel: {}", team_id, channel_name);
            // Return the channel ID from the first (and only) channel in the cached result
            entry.value.first().map(|channel| channel.channel_id.clone())
        }
    })
}

/// Cache single channel result for exact name lookup
pub fn cache_channel_by_name(team_id: &str, channel_name: &str, channel_record: ChannelRecord) {
    let key = channel_exact_cache_key(team_id, channel_name);
    let entry = CacheEntry::new(vec![channel_record.clone()]);
    CHANNELS_CACHE.insert(key, entry);
    tracing::info!("Cached channel {} for team {} exact name: {}", channel_record.channel_id, team_id, channel_name);
}

/// Clear all Teams caches (for testing/debugging)
#[allow(dead_code)]
pub fn clear_all_caches() {
    TEAMS_CACHE.clear();
    CHANNELS_CACHE.clear();
    tracing::debug!("All Teams caches cleared");
}

/// Get cache statistics for monitoring
#[allow(dead_code)]
pub fn cache_stats() -> (usize, usize) {
    (TEAMS_CACHE.len(), CHANNELS_CACHE.len())
}