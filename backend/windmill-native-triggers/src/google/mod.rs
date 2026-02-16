//! Google Native Trigger Module
//!
//! This module provides integration with Google services (Drive, Calendar)
//! push notification system to trigger Windmill scripts/flows when changes occur.
//!
//! ## Unified Architecture
//! A single "google" native trigger service handles both Drive and Calendar triggers.
//! The `trigger_type` field in `GoogleServiceConfig` determines which service to use.
//!
//! ## How it works:
//! 1. User configures a trigger with trigger_type (drive/calendar) and service-specific settings
//! 2. Windmill creates a "watch channel" via the appropriate Google API
//! 3. Google sends push notifications to Windmill's webhook when changes occur
//! 4. The webhook triggers the configured script/flow
//!
//! ## Important notes:
//! - Drive watch channels expire after max 24 hours
//! - Calendar watch channels expire after max 7 days
//! - Background sync job renews channels before expiration

use serde::{Deserialize, Serialize};

pub mod external;
pub mod routes;

pub use external::should_renew_channel;

/// Extracts `(google_resource_id, stop_url)` from a native trigger's service_config JSON.
/// Used by the `delete` method and tested independently.
pub fn parse_stop_channel_params(config: &serde_json::Value) -> (String, String) {
    let google_resource_id = config
        .get("googleResourceId")
        .and_then(|r| r.as_str())
        .map(String::from)
        .unwrap_or_default();

    let trigger_type = config
        .get("triggerType")
        .and_then(|t| t.as_str())
        .unwrap_or("drive");

    let stop_url = match trigger_type {
        "calendar" => format!("{}/channels/stop", endpoints::CALENDAR_API_BASE),
        _ => format!("{}/channels/stop", endpoints::DRIVE_API_BASE),
    };

    (google_resource_id, stop_url)
}

/// Handler struct for Google triggers (stateless, used for routing)
#[derive(Copy, Clone)]
pub struct Google;

/// Type of Google trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoogleTriggerType {
    Drive,
    Calendar,
}

impl std::fmt::Display for GoogleTriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoogleTriggerType::Drive => write!(f, "drive"),
            GoogleTriggerType::Calendar => write!(f, "calendar"),
        }
    }
}

/// User-provided configuration for a Google trigger.
/// The trigger_type determines which service-specific config is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleServiceConfig {
    /// The type of trigger (drive or calendar)
    pub trigger_type: GoogleTriggerType,

    // Drive-specific fields (only used when trigger_type = drive)
    /// The file ID to watch, or None for all changes (Drive only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    /// Human-readable name/path for display purposes (Drive only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_name: Option<String>,

    // Calendar-specific fields (only used when trigger_type = calendar)
    /// The calendar ID to watch (Calendar only, e.g., "primary")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
    /// Human-readable calendar name (Calendar only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_name: Option<String>,

    // Metadata from Google watch channel (set after creation, used for renewal/deletion)
    /// The resource ID assigned by Google for the watch channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_resource_id: Option<String>,
    /// Channel expiration time (Unix timestamp in milliseconds, as string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<String>,
}

impl GoogleServiceConfig {
    /// Returns the expiration duration for this trigger type in hours
    pub fn max_expiration_hours(&self) -> u64 {
        match self.trigger_type {
            GoogleTriggerType::Drive => 24,     // Google Drive: max 24 hours
            GoogleTriggerType::Calendar => 168, // Google Calendar: max 7 days
        }
    }
}

/// OAuth data structure shared by all Google services.
/// Stored encrypted in workspace_integrations table with service_name = 'google'.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleOAuthData {
    /// The OAuth access token for API requests
    pub access_token: String,
    /// The OAuth refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
    /// When the access token expires
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Google API endpoints
pub mod endpoints {
    /// Google Drive API v3 base URL
    pub const DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
    /// Google Calendar API v3 base URL
    pub const CALENDAR_API_BASE: &str = "https://www.googleapis.com/calendar/v3";
    /// Google OAuth2 token endpoint
    pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
    /// Google OAuth2 authorization endpoint
    pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
}

/// OAuth scopes for Google services
pub mod scopes {
    /// Read-only access to Google Drive files
    pub const DRIVE_READONLY: &str = "https://www.googleapis.com/auth/drive.readonly";
    /// Read-only access to Google Calendar
    pub const CALENDAR_READONLY: &str = "https://www.googleapis.com/auth/calendar.readonly";
    /// Events access to Google Calendar
    pub const CALENDAR_EVENTS: &str = "https://www.googleapis.com/auth/calendar.events";

    /// Returns all scopes needed for Google triggers (both Drive and Calendar)
    pub fn all_scopes() -> Vec<&'static str> {
        vec![DRIVE_READONLY, CALENDAR_READONLY, CALENDAR_EVENTS]
    }
}

/// Common response wrapper for Google API errors
#[derive(Debug, Deserialize)]
pub struct GoogleApiError {
    pub error: GoogleErrorDetails,
}

#[derive(Debug, Deserialize)]
pub struct GoogleErrorDetails {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub errors: Vec<GoogleErrorItem>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleErrorItem {
    pub domain: Option<String>,
    pub reason: Option<String>,
    pub message: Option<String>,
}

/// Google Watch Channel response (used by Drive and Calendar push notifications)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchChannel {
    /// Unique channel ID (we generate this as UUID)
    pub id: String,
    /// Resource ID assigned by Google
    pub resource_id: String,
    /// Resource URI being watched
    pub resource_uri: Option<String>,
    /// Channel expiration time (Unix timestamp in milliseconds)
    pub expiration: i64,
    /// Token for validation (optional)
    pub token: Option<String>,
}

/// Response from Google API when creating a watch channel
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWatchResponse {
    /// The channel ID we provided
    pub id: String,
    /// Resource ID assigned by Google
    pub resource_id: String,
    /// Resource URI being watched
    pub resource_uri: Option<String>,
    /// Channel expiration (Unix timestamp in milliseconds)
    pub expiration: String,
}

/// Request body for creating a watch channel
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchRequest {
    /// Unique channel ID (UUID)
    pub id: String,
    /// Type of delivery mechanism (always "web_hook")
    #[serde(rename = "type")]
    pub channel_type: String,
    /// The URL to receive notifications
    pub address: String,
    /// Optional token for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Optional expiration time in milliseconds (Google may adjust this)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<i64>,
}

impl WatchRequest {
    pub fn new(channel_id: String, webhook_url: String) -> Self {
        Self {
            id: channel_id,
            channel_type: "web_hook".to_string(),
            address: webhook_url,
            token: None,
            expiration: None,
        }
    }
}

/// Request body for stopping a watch channel
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopChannelRequest {
    pub id: String,
    pub resource_id: String,
}
