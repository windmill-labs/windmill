# Skill: Adding Native Trigger Services

This skill provides comprehensive guidance for adding new native trigger services to Windmill. Native triggers allow external services (like Nextcloud, Google Drive, etc.) to trigger Windmill scripts/flows via webhooks or push notifications.

## Architecture Overview

The native trigger system consists of:

1. **Database Layer** - PostgreSQL tables and enum types
2. **Backend Rust Implementation** - Core trait, handlers, and service modules in the `windmill-native-triggers` crate
3. **Frontend Svelte Components** - Configuration forms and UI components

### Key Files

| Component | Path |
|-----------|------|
| Core module with `External` trait | `backend/windmill-native-triggers/src/lib.rs` |
| Generic CRUD handlers | `backend/windmill-native-triggers/src/handler.rs` |
| Background sync logic | `backend/windmill-native-triggers/src/sync.rs` |
| OAuth/workspace integration | `backend/windmill-native-triggers/src/workspace_integrations.rs` |
| Re-export shim (windmill-api) | `backend/windmill-api/src/native_triggers/mod.rs` |
| TriggerKind enum | `backend/windmill-common/src/triggers.rs` |
| JobTriggerKind enum | `backend/windmill-common/src/jobs.rs` |
| Frontend service registry | `frontend/src/lib/components/triggers/native/utils.ts` |
| Frontend trigger utilities | `frontend/src/lib/components/triggers/utils.ts` |
| Trigger badges (icons + counts) | `frontend/src/lib/components/graph/renderers/triggers/TriggersBadge.svelte` |
| Workspace integrations UI | `frontend/src/lib/components/workspaceSettings/WorkspaceIntegrations.svelte` |
| OAuth config form component | `frontend/src/lib/components/workspaceSettings/OAuthClientConfig.svelte` |
| OpenAPI spec | `backend/windmill-api/openapi.yaml` |
| Reference: Nextcloud module | `backend/windmill-native-triggers/src/nextcloud/` |
| Reference: Google module | `backend/windmill-native-triggers/src/google/` |

### Crate Structure

The native trigger code lives in the `windmill-native-triggers` crate (`backend/windmill-native-triggers/`). The `windmill-api` crate re-exports everything via a shim:

```rust
// backend/windmill-api/src/native_triggers/mod.rs
pub use windmill_native_triggers::*;
```

All new service modules go in `backend/windmill-native-triggers/src/`.

---

## Core Concepts

### The `External` Trait

Every native trigger service implements the `External` trait defined in `lib.rs`:

```rust
#[async_trait]
pub trait External: Send + Sync + 'static {
    // Associated types:
    type ServiceConfig: Debug + DeserializeOwned + Serialize + Send + Sync;
    type TriggerData: Debug + Serialize + Send + Sync;
    type OAuthData: DeserializeOwned + Serialize + Clone + Send + Sync;
    type CreateResponse: DeserializeOwned + Send + Sync;

    // Constants:
    const SUPPORT_WEBHOOK: bool;
    const SERVICE_NAME: ServiceName;
    const DISPLAY_NAME: &'static str;
    const TOKEN_ENDPOINT: &'static str;
    const REFRESH_ENDPOINT: &'static str;
    const AUTH_ENDPOINT: &'static str;

    // Required methods:
    async fn create(&self, w_id, oauth_data, webhook_token, data, db, tx) -> Result<Self::CreateResponse>;
    async fn update(&self, w_id, oauth_data, external_id, webhook_token, data, db, tx) -> Result<serde_json::Value>;
    async fn get(&self, w_id, oauth_data, external_id, db, tx) -> Result<Self::TriggerData>;
    async fn delete(&self, w_id, oauth_data, external_id, db, tx) -> Result<()>;
    async fn exists(&self, w_id, oauth_data, external_id, db, tx) -> Result<bool>;
    async fn maintain_triggers(&self, db, workspace_id, triggers, oauth_data, synced, errors);
    fn external_id_and_metadata_from_response(&self, resp) -> (String, Option<serde_json::Value>);

    // Methods with defaults:
    async fn prepare_webhook(&self, db, w_id, headers, body, script_path, is_flow) -> Result<PushArgsOwned>;
    fn service_config_from_create_response(&self, data, resp) -> Option<serde_json::Value>;
    fn additional_routes(&self) -> axum::Router;
    async fn http_client_request<T, B>(&self, url, method, workspace_id, tx, db, headers, body) -> Result<T>;
}
```

Key design points:
- **`update()` returns `serde_json::Value`** - the resolved service_config to store. Each service is responsible for building the final config.
- **`maintain_triggers()`** - periodic background maintenance. Each service implements its own strategy (Nextcloud: reconcile with external state; Google: renew expiring channels).
- **No `list_all()` in the trait** - services that need it (Nextcloud) implement it privately; services that don't (Google) use different maintenance strategies.
- **No `get_external_id_from_trigger_data()` or `extract_service_config_from_trigger_data()`** - removed in favor of the `maintain_triggers` pattern.

### Create Lifecycle: Two Paths

The `create_native_trigger` handler in `handler.rs` supports two creation flows, controlled by `service_config_from_create_response()`:

**Path A: Short (Google pattern)** - `service_config_from_create_response()` returns `Some(config)`:
1. `create()` registers on external service
2. `external_id_and_metadata_from_response()` extracts the ID
3. `service_config_from_create_response()` builds the config directly from input data + response metadata
4. Stores trigger in DB -- done, no extra round-trip

Use this when the external_id is known before the create call (e.g., Google generates the channel_id as a UUID upfront and includes it in the webhook URL).

**Path B: Long (Nextcloud pattern)** - `service_config_from_create_response()` returns `None` (default):
1. `create()` registers on external service (webhook URL has no external_id yet)
2. `external_id_and_metadata_from_response()` extracts the ID
3. `update()` is called to fix the webhook URL with the now-known external_id
4. `update()` returns the resolved service_config
5. Stores trigger in DB

Use this when the external_id is assigned by the remote service and the webhook URL needs to be corrected after creation.

### OAuth Token Storage (Three-Table Pattern)

OAuth tokens are stored across three tables, NOT in `workspace_integrations.oauth_data` directly:

| Table | What's Stored |
|-------|---------------|
| `workspace_integrations` | `oauth_data` JSON with `base_url`, `client_id`, `client_secret`, `instance_shared` flag; `resource_path` pointing to the variable |
| `variable` | Encrypted `access_token` (at the path stored in `resource_path`), linked to `account` via `account` column |
| `account` | `refresh_token`, keyed by `workspace_id` + `client` (service name) + `is_workspace_integration = true` |

The `decrypt_oauth_data()` function in `lib.rs` assembles these into a unified struct:
```rust
pub struct OAuthConfig {
    pub base_url: String,
    pub access_token: String,      // decrypted from variable
    pub refresh_token: Option<String>, // from account table
    pub client_id: String,         // from oauth_data or instance settings
    pub client_secret: String,     // from oauth_data or instance settings
}
```

Instance-level sharing: when `oauth_data.instance_shared == true`, `client_id` and `client_secret` are read from global settings instead of workspace_integrations.

### URL Resolution

The `resolve_endpoint()` helper handles both absolute and relative OAuth URLs:

```rust
pub fn resolve_endpoint(base_url: &str, endpoint: &str) -> String {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()  // Google: absolute URLs
    } else {
        format!("{}{}", base_url, endpoint)  // Nextcloud: relative paths
    }
}
```

### ServiceName Methods

`ServiceName` is the central registry enum. Each variant must implement these match arms:

| Method | Purpose |
|--------|---------|
| `as_str()` | Lowercase identifier (e.g., `"google"`) |
| `as_trigger_kind()` | Maps to `TriggerKind` enum |
| `as_job_trigger_kind()` | Maps to `JobTriggerKind` enum |
| `token_endpoint()` | OAuth token endpoint (relative or absolute) |
| `auth_endpoint()` | OAuth authorization endpoint |
| `oauth_scopes()` | Space-separated OAuth scopes |
| `resource_type()` | Resource type for token storage (e.g., `"gworkspace"`) |
| `extra_auth_params()` | Extra OAuth params (e.g., Google needs `access_type=offline`, `prompt=consent`) |
| `integration_service()` | Maps to the workspace integration service (usually `*self`) |
| `TryFrom<String>` | Parse from string |
| `Display` | Delegates to `as_str()` |

---

## Step-by-Step Implementation Guide

### Step 1: Database Migration

Create a new migration file: `backend/migrations/YYYYMMDDHHMMSS_newservice_trigger.up.sql`

```sql
-- Add the service to the native_trigger_service enum
ALTER TYPE native_trigger_service ADD VALUE IF NOT EXISTS 'newservice';

-- Add to TRIGGER_KIND enum (used for trigger tracking)
ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'newservice';

-- Add to job_trigger_kind enum (used for job tracking)
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'newservice';
```

Also create the corresponding down migration.

### Step 2: Update windmill-common Enums

#### `backend/windmill-common/src/triggers.rs`

Add variant to `TriggerKind` enum, and update `to_key()` and `fmt()` implementations.

#### `backend/windmill-common/src/jobs.rs`

Add variant to `JobTriggerKind` enum and update the `Display` implementation.

### Step 3: Backend Service Module

Create a new directory: `backend/windmill-native-triggers/src/newservice/`

#### `mod.rs` - Type Definitions

```rust
use serde::{Deserialize, Serialize};

pub mod external;
// pub mod routes; // Only if you need additional service-specific routes

/// OAuth data deserialized from the three-table pattern.
/// The actual structure is built by decrypt_oauth_data() from variable + account + workspace_integrations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewServiceOAuthData {
    pub base_url: String,              // from workspace_integrations.oauth_data
    pub access_token: String,          // decrypted from variable table
    pub refresh_token: Option<String>, // from account table
    // Note: client_id and client_secret are in OAuthConfig, not here
    // unless the service needs them at runtime for API calls
}

/// Configuration provided by user when creating/updating a trigger.
/// Stored as JSON in native_trigger.service_config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServiceConfig {
    // Service-specific configuration fields
    pub folder_path: String,
    pub file_filter: Option<String>,
}

/// Data retrieved from the external service about a trigger.
/// Returned by the get() method and shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServiceTriggerData {
    pub folder_path: String,
    pub file_filter: Option<String>,
    // Fields that shouldn't affect service_config comparison should use #[serde(skip_serializing)]
}

/// Response from external service when creating a trigger/webhook.
#[derive(Debug, Deserialize)]
pub struct CreateTriggerResponse {
    pub id: String,
}

/// Handler struct (stateless, used for routing)
#[derive(Copy, Clone)]
pub struct NewService;
```

#### `external.rs` - External Trait Implementation

```rust
use async_trait::async_trait;
use reqwest::Method;
use sqlx::PgConnection;
use std::collections::HashMap;
use windmill_common::{
    error::{Error, Result},
    BASE_URL, DB,
};

use crate::{
    generate_webhook_service_url, External, NativeTrigger, NativeTriggerData, ServiceName,
    sync::{SyncError, TriggerSyncInfo},
};
use super::{NewService, NewServiceConfig, NewServiceOAuthData, NewServiceTriggerData, CreateTriggerResponse};

#[async_trait]
impl External for NewService {
    type ServiceConfig = NewServiceConfig;
    type TriggerData = NewServiceTriggerData;
    type OAuthData = NewServiceOAuthData;
    type CreateResponse = CreateTriggerResponse;

    const SERVICE_NAME: ServiceName = ServiceName::NewService;
    const DISPLAY_NAME: &'static str = "New Service";
    const SUPPORT_WEBHOOK: bool = true;
    const TOKEN_ENDPOINT: &'static str = "/oauth/token";
    const REFRESH_ENDPOINT: &'static str = "/oauth/token";
    const AUTH_ENDPOINT: &'static str = "/oauth/authorize";

    async fn create(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::CreateResponse> {
        let base_url = &*BASE_URL.read().await;

        // external_id is None during create (we get it from the response)
        let webhook_url = generate_webhook_service_url(
            base_url, w_id, &data.script_path, data.is_flow,
            None, Self::SERVICE_NAME, webhook_token,
        );

        let url = format!("{}/api/webhooks/create", oauth_data.base_url);
        let payload = serde_json::json!({
            "callback_url": webhook_url,
            "folder_path": data.service_config.folder_path,
        });

        let response: CreateTriggerResponse = self
            .http_client_request(&url, Method::POST, w_id, tx, db, None, Some(&payload))
            .await?;

        Ok(response)
    }

    /// Update returns the resolved service_config as JSON.
    /// For services using the update+get pattern, call self.get() and serialize.
    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<serde_json::Value> {
        let base_url = &*BASE_URL.read().await;

        let webhook_url = generate_webhook_service_url(
            base_url, w_id, &data.script_path, data.is_flow,
            Some(external_id), Self::SERVICE_NAME, webhook_token,
        );

        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);
        let payload = serde_json::json!({
            "callback_url": webhook_url,
            "folder_path": data.service_config.folder_path,
        });

        let _: serde_json::Value = self
            .http_client_request(&url, Method::PUT, w_id, tx, db, None, Some(&payload))
            .await?;

        // Fetch back the updated state to get the resolved config
        let trigger_data = self.get(w_id, oauth_data, external_id, db, tx).await?;
        serde_json::to_value(&trigger_data)
            .map_err(|e| Error::InternalErr(format!("Failed to serialize trigger data: {}", e)))
    }

    async fn get(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::TriggerData> {
        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);
        self.http_client_request::<_, ()>(&url, Method::GET, w_id, tx, db, None, None).await
    }

    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);
        let _: serde_json::Value = self
            .http_client_request::<_, ()>(&url, Method::DELETE, w_id, tx, db, None, None)
            .await
            .or_else(|e| match &e {
                Error::InternalErr(msg) if msg.contains("404") => Ok(serde_json::Value::Null),
                _ => Err(e),
            })?;
        Ok(())
    }

    async fn exists(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<bool> {
        match self.get(w_id, oauth_data, external_id, db, tx).await {
            Ok(_) => Ok(true),
            Err(Error::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Background maintenance. Choose the right pattern for your service:
    /// - For services with queryable external state: use reconcile_with_external_state()
    /// - For channel-based services with expiration: implement renewal logic
    async fn maintain_triggers(
        &self,
        db: &DB,
        workspace_id: &str,
        triggers: &[NativeTrigger],
        oauth_data: &Self::OAuthData,
        synced: &mut Vec<TriggerSyncInfo>,
        errors: &mut Vec<SyncError>,
    ) {
        // Option A: Reconcile with external state (Nextcloud pattern)
        // Fetch all triggers from external service and compare with DB
        let external_triggers = match self.list_all(workspace_id, oauth_data, db).await {
            Ok(triggers) => triggers,
            Err(e) => {
                errors.push(SyncError {
                    resource_path: format!("workspace:{}", workspace_id),
                    error_message: format!("Failed to list triggers: {}", e),
                    error_type: "api_error".to_string(),
                });
                return;
            }
        };

        // Convert to (external_id, config_json) pairs
        let external_pairs: Vec<(String, serde_json::Value)> = external_triggers
            .into_iter()
            .map(|t| (t.id.clone(), serde_json::to_value(&t).unwrap_or_default()))
            .collect();

        crate::sync::reconcile_with_external_state(
            db, workspace_id, Self::SERVICE_NAME, triggers, &external_pairs, synced, errors,
        ).await;
    }

    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.clone(), None)
    }

    // service_config_from_create_response: NOT overridden (returns None).
    // This means the handler uses the update+get pattern after create.
    // Override and return Some(...) to skip the update+get cycle (Google pattern).
}

impl NewService {
    /// Private helper to list all triggers from the external service.
    async fn list_all(
        &self,
        w_id: &str,
        oauth_data: &<Self as External>::OAuthData,
        db: &DB,
    ) -> Result<Vec<<Self as External>::TriggerData>> {
        // Implementation depends on the external service's API
        todo!()
    }
}
```

### Step 4: Update lib.rs Registry

In `backend/windmill-native-triggers/src/lib.rs`:

```rust
// Service modules - add new services here:
#[cfg(feature = "native_trigger")]
pub mod newservice;  // <-- Add this

// ServiceName enum - add variant:
pub enum ServiceName {
    Nextcloud,
    Google,
    NewService,  // <-- Add this
}

// Then add match arms in ALL ServiceName methods:
// as_str(), as_trigger_kind(), as_job_trigger_kind(), token_endpoint(),
// auth_endpoint(), oauth_scopes(), resource_type(), extra_auth_params(),
// integration_service(), TryFrom<String>, Display
```

### Step 5: Update handler.rs Routes

In `backend/windmill-native-triggers/src/handler.rs`:

```rust
pub fn generate_native_trigger_routers() -> Router {
    // ...
    #[cfg(feature = "native_trigger")]
    {
        use crate::newservice::NewService;
        return router
            .nest("/nextcloud", service_routes(NextCloud))
            .nest("/google", service_routes(Google))
            .nest("/newservice", service_routes(NewService));  // <-- Add this
    }
    // ...
}
```

### Step 6: Update sync.rs

In `backend/windmill-native-triggers/src/sync.rs`:

```rust
pub async fn sync_all_triggers(db: &DB) -> Result<BackgroundSyncResult> {
    // ...
    #[cfg(feature = "native_trigger")]
    {
        use crate::newservice::NewService;

        // ... existing service syncs ...

        // New service sync
        let (service_name, result) = sync_service_triggers(db, NewService).await;
        total_synced += result.synced_triggers.len();
        total_errors += result.errors.len();
        service_results.insert(service_name, result);
    }
    // ...
}
```

### Step 7: Frontend Service Registry

In `frontend/src/lib/components/triggers/native/utils.ts`:

Add to `NATIVE_TRIGGER_SERVICES`, `getTriggerIconName()`, and `getServiceIcon()`.

### Step 8: Frontend Trigger Form Component

Create: `frontend/src/lib/components/triggers/native/services/newservice/NewServiceTriggerForm.svelte`

### Step 9: Frontend Icon Component

Create: `frontend/src/lib/components/icons/NewServiceIcon.svelte`

### Step 10: Update NativeTriggerEditor

Check `frontend/src/lib/components/triggers/native/NativeTriggerEditor.svelte` to ensure it dynamically loads form components based on service name.

### Step 11: Workspace Integration UI

Add your service to the `supportedServices` map in `frontend/src/lib/components/workspaceSettings/WorkspaceIntegrations.svelte`:

```typescript
const supportedServices: Record<string, ServiceConfig> = {
    // ... existing services ...
    newservice: {
        name: 'newservice',
        displayName: 'New Service',
        description: 'Connect to New Service for triggers',
        icon: NewServiceIcon,
        docsUrl: 'https://www.windmill.dev/docs/integrations/newservice',
        requiresBaseUrl: false,  // false for cloud services, true for self-hosted
        setupInstructions: [
            'Step 1: Create an OAuth app on the service',
            'Step 2: Configure the redirect URI shown below',
            'Step 3: Enter the client credentials below'
        ]
    }
}
```

### Step 12: Update `frontend/src/lib/components/triggers/utils.ts`

Update ALL of these maps/functions:
1. `triggerIconMap` - import and add icon
2. `triggerDisplayNamesMap` - add display name
3. `triggerTypeOrder` in `sortTriggers()` - add type
4. `getLightConfig()` - add case for your service
5. `getTriggerLabel()` - add case for your service
6. `jobTriggerKinds` - add to array
7. `countPropertyMap` - add count property
8. `triggerSaveFunctions` - add save function

### Step 13: Update TriggersBadge Component

In `frontend/src/lib/components/graph/renderers/triggers/TriggersBadge.svelte`:

1. Import the icon
2. Add to `baseConfig` with `countKey` (the dynamic `availableNativeServices` loop does NOT set `countKey`)
3. Add to the `allTypes` array

### Step 14: Update TriggersWrapper.svelte

In `frontend/src/lib/components/triggers/TriggersWrapper.svelte`:

Add a `{:else if selectedTrigger.type === 'yourservice'}` case that renders `<NativeTriggersPanel service="yourservice" ...>` with the same props pattern as the existing native trigger cases (e.g., `nextcloud`).

### Step 15: Update AddTriggersButton.svelte

In `frontend/src/lib/components/triggers/AddTriggersButton.svelte`:

1. Add `yourserviceAvailable` state variable
2. Add `setYourserviceState()` async function using `isServiceAvailable('yourservice', $workspaceStore!)`
3. Call it at module level
4. Add a dropdown entry to `addTriggerItems` with `hidden: !yourserviceAvailable`

### Step 16: Update TriggersEditor.svelte Delete Handling

In `frontend/src/lib/components/triggers/TriggersEditor.svelte`:

Add your service to the `nativeTriggerServices` map in `deleteDeployedTrigger()`. Native triggers use `NativeTriggerService.deleteNativeTrigger({ workspace, serviceName, externalId })` instead of the standard `path`-based delete.

### Step 17: Update OpenAPI Spec and Regenerate Types

Add to `JobTriggerKind` enum in `backend/windmill-api/openapi.yaml`, then:

```bash
cd frontend && npm run generate-backend-client
```

---

## Special Patterns

### Unified Service with `trigger_type` (Google Pattern)

When a single service handles multiple trigger types (e.g., Google Drive + Calendar share OAuth and API patterns), use a single `ServiceName` variant with a discriminator field:

```rust
pub enum GoogleTriggerType { Drive, Calendar }

pub struct GoogleServiceConfig {
    pub trigger_type: GoogleTriggerType,
    // Drive-specific fields (only used when trigger_type = Drive)
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    // Calendar-specific fields (only used when trigger_type = Calendar)
    pub calendar_id: Option<String>,
    pub calendar_name: Option<String>,
    // Metadata set after creation
    pub google_resource_id: Option<String>,
    pub expiration: Option<String>,
}
```

Branch in trait methods based on `trigger_type`. Frontend uses a `ToggleButtonGroup` to switch between types. This keeps the codebase simpler (one service, one OAuth flow, one set of routes).

See `backend/windmill-native-triggers/src/google/` for the reference implementation.

### Skipping update+get After Create (Google Pattern)

Override `service_config_from_create_response()` to return `Some(config)` when the external_id is known before the create call:

```rust
fn service_config_from_create_response(
    &self,
    data: &NativeTriggerData<Self::ServiceConfig>,
    resp: &Self::CreateResponse,
) -> Option<serde_json::Value> {
    // Clone input config, add metadata from response
    let mut config = data.service_config.clone();
    config.google_resource_id = Some(resp.resource_id.clone());
    config.expiration = Some(resp.expiration.clone());
    Some(serde_json::to_value(&config).unwrap())
}
```

### Services with Absolute OAuth Endpoints (Google)

Unlike self-hosted services where OAuth endpoints are relative paths appended to `base_url`, services like Google have absolute URLs:

```rust
// Nextcloud: relative paths
ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",
// Google: absolute URLs
ServiceName::Google => "https://oauth2.googleapis.com/token",
```

The `resolve_endpoint()` function handles both. For services with absolute endpoints:
- `base_url` can be empty
- `requiresBaseUrl: false` in the frontend workspace integration config
- Add `extra_auth_params()` if needed (Google requires `access_type=offline` and `prompt=consent`)

### Channel-Based Push Notifications with Renewal (Google Pattern)

For services using expiring watch channels instead of persistent webhooks:

1. Store expiration in `service_config` (as part of `ServiceConfig`)
2. In `maintain_triggers()`, implement renewal logic instead of using `reconcile_with_external_state()`:
   ```rust
   async fn maintain_triggers(&self, db, workspace_id, triggers, oauth_data, synced, errors) {
       for trigger in triggers {
           if should_renew_channel(trigger) {
               self.renew_channel(db, trigger, oauth_data).await;
           }
       }
   }
   ```
3. Renewal: best-effort stop old channel, create new one with same external_id, update service_config with new expiration
4. Google example: Drive channels expire in 24h (renew when <1h left), Calendar channels expire in 7 days (renew when <1 day left)

### reconcile_with_external_state (Nextcloud Pattern)

The reusable function in `sync.rs` compares external triggers with DB state:
- Triggers missing externally: sets error "Trigger no longer exists on external service"
- Triggers present externally: clears errors, updates service_config if it differs

Usage in `maintain_triggers()`:
```rust
let external_pairs: Vec<(String, serde_json::Value)> = /* fetch from external */;
crate::sync::reconcile_with_external_state(
    db, workspace_id, Self::SERVICE_NAME, triggers, &external_pairs, synced, errors,
).await;
```

### Webhook Payload Processing

Override `prepare_webhook()` to parse service-specific payloads into script/flow args:

```rust
async fn prepare_webhook(&self, db, w_id, headers, body, script_path, is_flow) -> Result<PushArgsOwned> {
    let mut args = HashMap::new();
    args.insert("event_type".to_string(), Box::new(headers.get("x-event-type").cloned()) as _);
    args.insert("payload".to_string(), Box::new(serde_json::from_str::<serde_json::Value>(&body)?) as _);
    Ok(PushArgsOwned { extra: None, args })
}
```

Then register in `prepare_native_trigger_args()` in `lib.rs`:
```rust
pub async fn prepare_native_trigger_args(service_name, db, w_id, headers, body) -> Result<Option<PushArgsOwned>> {
    match service_name {
        ServiceName::Google => { /* ... */ Ok(Some(args)) }
        ServiceName::NewService => { /* ... */ Ok(Some(args)) }
        ServiceName::Nextcloud => Ok(None), // Uses default body parsing
    }
}
```

### Instance-Level OAuth Credentials

When `workspace_integrations.oauth_data.instance_shared == true`, `decrypt_oauth_data()` reads `client_id` and `client_secret` from instance-level global settings instead of workspace-level. This allows admins to share OAuth app credentials across workspaces.

The frontend handles this via the `generate_instance_connect_url` endpoint in `workspace_integrations.rs`.

---

## Testing Checklist

- [ ] Database migration runs successfully
- [ ] `cargo check -p windmill-native-triggers --features native_trigger` passes
- [ ] `npx svelte-check --threshold error` passes (in frontend/)
- [ ] Service appears in workspace integrations list
- [ ] OAuth flow completes successfully
- [ ] Can create a new trigger
- [ ] Can view trigger details
- [ ] Can update trigger configuration
- [ ] Can delete trigger
- [ ] Webhook receives and processes payloads
- [ ] Background sync works correctly (reconciliation or channel renewal)
- [ ] Error handling works (expired tokens, service unavailable)

---

## Reference Implementations

### Nextcloud (Self-Hosted, Update+Get Pattern)

| File | Purpose |
|------|---------|
| `nextcloud/mod.rs` | Types: NextCloudOAuthData, NextcloudServiceConfig, NextCloudTriggerData |
| `nextcloud/external.rs` | External trait: uses update+get pattern, reconcile_with_external_state for sync |
| `nextcloud/routes.rs` | Additional route: `GET /events` |

Key patterns: relative OAuth endpoints, base_url required, list_all + reconcile for sync, update returns JSON from get().

### Google (Cloud, Unified Service, Short Create)

| File | Purpose |
|------|---------|
| `google/mod.rs` | Types: GoogleServiceConfig with trigger_type discriminator, GoogleTriggerType enum |
| `google/external.rs` | External trait: overrides service_config_from_create_response, channel renewal for sync |
| `google/routes.rs` | Additional routes: `GET /calendars`, `GET /drive/files`, `GET /drive/shared_drives` |

Key patterns: absolute OAuth endpoints, empty base_url, trigger_type for Drive/Calendar, expiring watch channels with renewal, service_config_from_create_response skips update+get, get() reconstructs data from stored service_config (no external "get channel" API).
