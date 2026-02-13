# Skill: Adding Native Trigger Services

This skill provides comprehensive guidance for adding new native trigger services to Windmill. Native triggers allow external services (like Nextcloud, Google Drive, etc.) to trigger Windmill scripts/flows via webhooks or push notifications.

## Architecture Overview

The native trigger system consists of:

1. **Database Layer** - PostgreSQL tables and enum types
2. **Backend Rust Implementation** - Core trait, handlers, and service modules
3. **Frontend Svelte Components** - Configuration forms and UI components

### Key Files

| Component | Path |
|-----------|------|
| Core module with `External` trait | `backend/windmill-api/src/native_triggers/mod.rs` |
| Generic CRUD handlers | `backend/windmill-api/src/native_triggers/handler.rs` |
| Background sync logic | `backend/windmill-api/src/native_triggers/sync.rs` |
| OAuth/workspace integration | `backend/windmill-api/src/native_triggers/workspace_integrations.rs` |
| TriggerKind enum | `backend/windmill-common/src/triggers.rs` |
| JobTriggerKind enum | `backend/windmill-common/src/jobs.rs` |
| Frontend service registry | `frontend/src/lib/components/triggers/native/utils.ts` |
| Frontend trigger utilities | `frontend/src/lib/components/triggers/utils.ts` |
| Trigger badges (icons + counts) | `frontend/src/lib/components/graph/renderers/triggers/TriggersBadge.svelte` |
| Workspace integrations UI | `frontend/src/lib/components/workspaceSettings/WorkspaceIntegrations.svelte` |
| OAuth config form component | `frontend/src/lib/components/workspaceSettings/OAuthClientConfig.svelte` |
| OpenAPI spec | `backend/windmill-api/openapi.yaml` |
| Reference: Nextcloud module | `backend/windmill-api/src/native_triggers/nextcloud/` |
| Reference: Google module | `backend/windmill-api/src/native_triggers/google/` |

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

Add variant to `TriggerKind` enum:

```rust
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, EnumIter)]
#[sqlx(type_name = "TRIGGER_KIND", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    // ... existing variants ...
    NewService,  // <-- Add this
}
```

Update `to_key()` and `fmt()` implementations:

```rust
impl TriggerKind {
    pub fn to_key(&self) -> String {
        match self {
            // ... existing match arms ...
            TriggerKind::NewService => "newservice".to_string(),
        }
    }
}

impl fmt::Display for TriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            // ... existing match arms ...
            TriggerKind::NewService => "newservice",
        };
        write!(f, "{}", s)
    }
}
```

#### `backend/windmill-common/src/jobs.rs`

Add variant to `JobTriggerKind` enum:

```rust
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone)]
#[sqlx(type_name = "job_trigger_kind", rename_all = "snake_case")]
pub enum JobTriggerKind {
    // ... existing variants ...
    NewService,  // <-- Add this
}
```

Update the `Display` implementation:

```rust
impl std::fmt::Display for JobTriggerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            // ... existing match arms ...
            JobTriggerKind::NewService => "newservice",
        };
        write!(f, "{}", kind)
    }
}
```

### Step 3: Backend Service Module

Create a new directory: `backend/windmill-api/src/native_triggers/newservice/`

#### `mod.rs` - Type Definitions

```rust
use serde::{Deserialize, Serialize};

pub mod external;
// pub mod routes; // Only if you need additional service-specific routes

/// OAuth data structure for the service.
/// Stored encrypted in workspace_integrations table.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NewServiceOAuthData {
    pub base_url: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    // Add service-specific OAuth fields (e.g., client_id, client_secret if needed at runtime)
}

/// Configuration provided by user when creating a trigger.
/// This is what the user configures in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServiceConfig {
    // Service-specific configuration fields
    // Example for a file watching service:
    pub folder_path: String,
    pub file_filter: Option<String>,
    pub include_subfolders: bool,
}

/// Data retrieved from the external service about a trigger.
/// Used for sync operations to compare external state vs Windmill state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServiceTriggerData {
    #[serde(skip_serializing)]  // Skip fields that shouldn't be part of service_config comparison
    pub id: String,             // External service's trigger ID
    #[serde(skip_serializing)]
    pub webhook_url: String,    // The callback URL registered
    // Include fields that should be synced (e.g., filter settings)
    pub folder_path: String,
    pub file_filter: Option<String>,
    pub include_subfolders: bool,
}

/// Response from external service when creating a trigger/webhook.
#[derive(Debug, Deserialize)]
pub struct CreateTriggerResponse {
    pub id: String,
    // Other fields returned by the API
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

use crate::native_triggers::{
    generate_webhook_service_url, External, NativeTriggerData, ServiceName,
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
    const SUPPORT_WEBHOOK: bool = true;  // true if service calls back to Windmill
    const TOKEN_ENDPOINT: &'static str = "/oauth/token";      // OAuth token endpoint path
    const REFRESH_ENDPOINT: &'static str = "/oauth/token";    // Token refresh endpoint path

    /// Create a new trigger on the external service.
    /// Called when user creates a native trigger in Windmill.
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

        // Generate the webhook URL that external service will call
        // external_id is None during create (we get it from the response)
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            None,  // No external_id yet
            Self::SERVICE_NAME,
            webhook_token,
        );

        // Build the request to external service
        let url = format!("{}/api/webhooks/create", oauth_data.base_url);

        let payload = serde_json::json!({
            "callback_url": webhook_url,
            "folder_path": data.service_config.folder_path,
            "file_filter": data.service_config.file_filter,
            "include_subfolders": data.service_config.include_subfolders,
        });

        // Use http_client_request for automatic token refresh on 401/403
        let response: CreateTriggerResponse = self
            .http_client_request(
                &url,
                Method::POST,
                w_id,
                tx,
                db,
                None,  // Optional custom headers
                Some(&payload),
            )
            .await?;

        Ok(response)
    }

    /// Update an existing trigger on the external service.
    /// Called when user updates a native trigger in Windmill.
    async fn update(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        webhook_token: &str,
        data: &NativeTriggerData<Self::ServiceConfig>,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        let base_url = &*BASE_URL.read().await;

        // Include external_id in webhook URL during update
        let webhook_url = generate_webhook_service_url(
            base_url,
            w_id,
            &data.script_path,
            data.is_flow,
            Some(external_id),
            Self::SERVICE_NAME,
            webhook_token,
        );

        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);

        let payload = serde_json::json!({
            "callback_url": webhook_url,
            "folder_path": data.service_config.folder_path,
            "file_filter": data.service_config.file_filter,
            "include_subfolders": data.service_config.include_subfolders,
        });

        let _: serde_json::Value = self
            .http_client_request(&url, Method::PUT, w_id, tx, db, None, Some(&payload))
            .await?;

        Ok(())
    }

    /// Get trigger data from the external service.
    /// Used when viewing a trigger and during sync operations.
    async fn get(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Self::TriggerData> {
        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);

        let response: Self::TriggerData = self
            .http_client_request::<_, ()>(&url, Method::GET, w_id, tx, db, None, None)
            .await?;

        Ok(response)
    }

    /// Delete a trigger from the external service.
    /// Called when user deletes a native trigger in Windmill.
    async fn delete(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        external_id: &str,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<()> {
        let url = format!("{}/api/webhooks/{}", oauth_data.base_url, external_id);

        // Handle 404 gracefully (trigger may already be deleted on external service)
        let _: serde_json::Value = self
            .http_client_request::<_, ()>(&url, Method::DELETE, w_id, tx, db, None, None)
            .await
            .or_else(|e| match &e {
                Error::InternalErr(msg) if msg.contains("404") => Ok(serde_json::Value::Null),
                _ => Err(e),
            })?;

        Ok(())
    }

    /// Check if a trigger exists on the external service.
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

    /// List all triggers from the external service.
    /// Used during background sync to detect config drift.
    async fn list_all(
        &self,
        w_id: &str,
        oauth_data: &Self::OAuthData,
        db: &DB,
        tx: &mut PgConnection,
    ) -> Result<Vec<Self::TriggerData>> {
        let url = format!("{}/api/webhooks", oauth_data.base_url);

        let response: Vec<Self::TriggerData> = self
            .http_client_request::<_, ()>(&url, Method::GET, w_id, tx, db, None, None)
            .await?;

        Ok(response)
    }

    /// Extract external_id and optional metadata from create response.
    fn external_id_and_metadata_from_response(
        &self,
        resp: &Self::CreateResponse,
    ) -> (String, Option<serde_json::Value>) {
        (resp.id.clone(), None)
    }

    /// Extract external_id from trigger data (used during sync).
    fn get_external_id_from_trigger_data(&self, data: &Self::TriggerData) -> String {
        data.id.clone()
    }

    /// Optional: Add service-specific routes (e.g., for listing available events).
    fn additional_routes(&self) -> axum::Router {
        // Return empty router if no additional routes needed
        axum::Router::new()

        // Or add custom routes:
        // routes::newservice_routes(self.clone())
    }

    /// Optional: Custom webhook payload processing.
    /// Default implementation returns empty args; override for service-specific parsing.
    async fn prepare_webhook(
        &self,
        _db: &DB,
        _w_id: &str,
        headers: HashMap<String, String>,
        body: String,
        _script_path: &str,
        _is_flow: bool,
    ) -> Result<windmill_queue::PushArgsOwned> {
        // Parse the webhook payload from the external service
        let payload: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| Error::BadRequest(format!("Invalid webhook payload: {}", e)))?;

        let mut args = HashMap::new();
        args.insert("payload".to_string(), Box::new(payload) as _);

        Ok(windmill_queue::PushArgsOwned { extra: None, args })
    }
}
```

### Step 4: Update mod.rs Registry

In `backend/windmill-api/src/native_triggers/mod.rs`:

```rust
// Service modules - add new services here:
#[cfg(feature = "native_trigger")]
pub mod nextcloud;
#[cfg(feature = "native_trigger")]
pub mod newservice;  // <-- Add this

/// Enum of all supported native trigger services.
#[derive(EnumIter, sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[sqlx(type_name = "native_trigger_service", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceName {
    Nextcloud,
    NewService,  // <-- Add this
}

impl TryFrom<String> for ServiceName {
    // ...
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let service = match value.as_str() {
            "nextcloud" => ServiceName::Nextcloud,
            "newservice" => ServiceName::NewService,  // <-- Add this
            _ => { /* ... */ }
        };
        Ok(service)
    }
}

impl ServiceName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "nextcloud",
            ServiceName::NewService => "newservice",  // <-- Add this
        }
    }

    pub fn as_trigger_kind(&self) -> TriggerKind {
        match self {
            ServiceName::Nextcloud => TriggerKind::Nextcloud,
            ServiceName::NewService => TriggerKind::NewService,  // <-- Add this
        }
    }

    pub fn as_job_trigger_kind(&self) -> windmill_common::jobs::JobTriggerKind {
        match self {
            ServiceName::Nextcloud => windmill_common::jobs::JobTriggerKind::Nextcloud,
            ServiceName::NewService => windmill_common::jobs::JobTriggerKind::NewService,  // <-- Add this
        }
    }

    pub fn token_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",
            ServiceName::NewService => "/oauth/token",  // <-- Add this
        }
    }

    pub fn auth_endpoint(&self) -> &'static str {
        match self {
            ServiceName::Nextcloud => "/apps/oauth2/authorize",
            ServiceName::NewService => "/oauth/authorize",  // <-- Add this
        }
    }
}
```

### Step 5: Update handler.rs Routes

In `backend/windmill-api/src/native_triggers/handler.rs`:

```rust
pub fn generate_native_trigger_routers() -> Router {
    let router = Router::new();

    #[cfg(feature = "native_trigger")]
    {
        use crate::native_triggers::nextcloud::NextCloud;
        use crate::native_triggers::newservice::NewService;  // <-- Add import

        return router
            .nest("/nextcloud", service_routes(NextCloud))
            .nest("/newservice", service_routes(NewService));  // <-- Add route
    }

    #[cfg(not(feature = "native_trigger"))]
    {
        router
    }
}
```

### Step 6: Update sync.rs

In `backend/windmill-api/src/native_triggers/sync.rs`:

```rust
pub async fn sync_all_triggers(db: &DB) -> Result<BackgroundSyncResult> {
    // ...

    #[cfg(feature = "native_trigger")]
    {
        use crate::native_triggers::nextcloud::NextCloud;
        use crate::native_triggers::newservice::NewService;  // <-- Add import

        // Nextcloud sync
        let (service_name, result) = sync_service_triggers(db, NextCloud).await;
        total_synced += result.synced_triggers.len();
        total_errors += result.errors.len();
        service_results.insert(service_name, result);

        // New service sync  <-- Add this block
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

```typescript
export const NATIVE_TRIGGER_SERVICES: Record<NativeServiceName, NativeTriggerConfig> = {
    nextcloud: {
        serviceDisplayName: 'Nextcloud',
        serviceKey: 'nextcloud',
        supportsSync: true,
        supportsFetchConfig: true,
        isCloudCompatible: true,
        templates: {
            script: '/scripts/add?hub=hub%2F28115',
            flow: '/flows/add?hub=73'
        }
    },
    newservice: {  // <-- Add this
        serviceDisplayName: 'New Service',
        serviceKey: 'newservice',
        supportsSync: true,
        supportsFetchConfig: true,
        isCloudCompatible: true,  // false if requires on-prem deployment
        templates: {
            // Optional hub template links
        }
    }
}

export function getTriggerIconName(service: NativeServiceName): string {
    switch (service) {
        case 'nextcloud':
            return 'NextcloudIcon'
        case 'newservice':  // <-- Add this
            return 'NewServiceIcon'
        default:
            return 'NextcloudIcon'
    }
}

export async function getServiceIcon(service: NativeServiceName): Promise<any> {
    switch (service) {
        case 'nextcloud':
            return (await import('$lib/components/icons/NextcloudIcon.svelte')).default
        case 'newservice':  // <-- Add this
            return (await import('$lib/components/icons/NewServiceIcon.svelte')).default
    }
}
```

### Step 8: Frontend Trigger Form Component

Create: `frontend/src/lib/components/triggers/native/services/newservice/NewServiceTriggerForm.svelte`

```svelte
<script lang="ts">
    import { workspaceStore } from '$lib/stores'
    import SchemaForm from '$lib/components/SchemaForm.svelte'
    import Section from '$lib/components/Section.svelte'
    import { Loader2 } from 'lucide-svelte'

    interface Props {
        serviceConfig: Record<string, any>
        errors: Record<string, string>
        disabled?: boolean
        externalData?: any
        loading: boolean
    }

    let {
        serviceConfig = $bindable(),
        errors = $bindable(),
        disabled = false,
        externalData = undefined,
        loading = $bindable()
    }: Props = $props()

    // Define the schema for the service configuration form
    const serviceSchema = {
        type: 'object',
        properties: {
            folder_path: {
                type: 'string',
                title: 'Folder Path',
                description: 'The folder to watch for changes'
            },
            file_filter: {
                type: 'string',
                title: 'File Filter',
                description: 'Optional glob pattern to filter files (e.g., *.pdf)'
            },
            include_subfolders: {
                type: 'boolean',
                title: 'Include Subfolders',
                description: 'Whether to watch subfolders recursively',
                default: false
            }
        },
        required: ['folder_path']
    }

    export function validate(): Record<string, string> {
        let serviceErrors: Record<string, string> = {}

        if (!serviceConfig.folder_path?.trim()) {
            serviceErrors.folder_path = 'Folder path is required'
        }

        return serviceErrors
    }

    let externalDataApplied = $state(false)

    // Apply external data when editing existing trigger
    $effect(() => {
        if (externalData && !externalDataApplied) {
            serviceConfig = { ...serviceConfig, ...externalData }
            externalDataApplied = true
        }
    })

    $effect(() => {
        if (!externalData) {
            externalDataApplied = false
        }
    })
</script>

<Section label="New Service Configuration">
    {#if loading}
        <div class="flex items-center gap-2 text-secondary text-xs">
            <Loader2 class="animate-spin" size={16} />
            Loading configuration...
        </div>
    {:else}
        <SchemaForm
            schema={serviceSchema}
            bind:args={serviceConfig}
            isValid={true}
            compact={true}
            prettifyHeader={true}
            {disabled}
        />
    {/if}
</Section>
```

### Step 9: Frontend Icon Component (Optional)

Create: `frontend/src/lib/components/icons/NewServiceIcon.svelte`

```svelte
<script lang="ts">
    interface Props {
        size?: number
        class?: string
    }

    let { size = 24, class: className = '' }: Props = $props()
</script>

<svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    class={className}
>
    <!-- Add your SVG path here -->
    <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2"/>
</svg>
```

### Step 10: Update NativeTriggerEditor (if needed)

Check `frontend/src/lib/components/triggers/native/NativeTriggerEditor.svelte` to ensure it dynamically loads form components based on service name. The existing pattern should work if you follow the naming convention:

```typescript
// The editor typically imports forms dynamically:
const FormComponent = await import(`./services/${service}/${ServiceName}TriggerForm.svelte`)
```

---

## Special Patterns

### Unified Service with `trigger_type` (e.g., Google)

When a single service handles multiple trigger types (e.g., Google Drive + Calendar share the same OAuth and API patterns), use a **unified service** approach instead of separate `ServiceName` variants:

1. **Single `ServiceName` variant**: Use `Google` (not `GoogleDrive`/`GoogleCalendar`)
2. **Discriminator field**: Add `trigger_type` to `ServiceConfig`:
   ```rust
   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   #[serde(rename_all = "snake_case")]
   pub enum GoogleTriggerType {
       Drive,
       Calendar,
   }

   pub struct GoogleServiceConfig {
       pub trigger_type: GoogleTriggerType,
       // Drive-specific fields (only used when trigger_type = drive)
       pub resource_id: Option<String>,
       // Calendar-specific fields (only used when trigger_type = calendar)
       pub calendar_id: Option<String>,
   }
   ```
3. **Branching in External trait methods**: Route to the correct API based on trigger_type:
   ```rust
   async fn create(&self, ...) -> Result<Self::CreateResponse> {
       match data.service_config.trigger_type {
           GoogleTriggerType::Drive => self.create_drive_watch(...).await,
           GoogleTriggerType::Calendar => self.create_calendar_watch(...).await,
       }
   }
   ```
4. **Frontend**: Use a `ToggleButtonGroup` to switch between trigger types in the form
5. **Shared OAuth**: Both trigger types use the same `workspace_integrations` entry

This approach keeps the codebase simpler (one service, one OAuth flow, one set of routes) while supporting multiple trigger types.

See `backend/windmill-api/src/native_triggers/google/` for the reference implementation of this pattern.

### Services with Absolute OAuth Endpoints (e.g., Google)

Unlike self-hosted services (e.g., Nextcloud) where OAuth endpoints are relative paths appended to `base_url`, some services like Google have absolute OAuth endpoint URLs:

```rust
// Nextcloud: relative paths appended to base_url
ServiceName::Nextcloud => "/apps/oauth2/api/v1/token",

// Google: absolute URLs (base_url is empty or unused for token endpoint)
ServiceName::Google => "https://oauth2.googleapis.com/token",
```

Key considerations:
- The `workspace_integrations.rs` `build_authorization_url()` and `build_native_oauth_client()` functions construct URLs differently for absolute vs relative endpoints
- For Google OAuth, you need `access_type=offline` and `prompt=consent` parameters to get refresh tokens
- The `OAuthConfig.base_url` field may be empty for services with absolute endpoints
- Google OAuth scopes are space-separated URLs (e.g., `https://www.googleapis.com/auth/drive.readonly`)

### Channel-Based Push Notifications (e.g., Google APIs)

For services that use expiring "watch channels" instead of persistent webhooks:

1. **Store expiration in service_config**:
   ```rust
   pub struct GoogleDriveTriggerData {
       pub channel_id: String,
       pub resource_id: String,
       pub expiration: i64,  // Unix timestamp
   }
   ```

2. **Add renewal logic in sync.rs**:
   ```rust
   // In sync_workspace_triggers, check for expiring channels
   let renewal_threshold = Utc::now() + Duration::hours(1);
   if trigger.expiration < renewal_threshold.timestamp() {
       // Renew the channel before it expires
       handler.renew_channel(w_id, oauth_data, external_id, db, tx).await?;
   }
   ```

3. **Implement channel renewal in External trait** (add method if needed):
   ```rust
   async fn renew_channel(...) -> Result<()> {
       // Stop old channel, create new one
   }
   ```

### Webhook Payload Processing

Override `prepare_webhook` to parse service-specific payloads:

```rust
async fn prepare_webhook(
    &self,
    _db: &DB,
    _w_id: &str,
    headers: HashMap<String, String>,
    body: String,
    _script_path: &str,
    _is_flow: bool,
) -> Result<PushArgsOwned> {
    // Parse service-specific payload structure
    let payload: MyServicePayload = serde_json::from_str(&body)?;

    let mut args = HashMap::new();
    args.insert("event_type".to_string(), Box::new(payload.event_type) as _);
    args.insert("data".to_string(), Box::new(payload.data) as _);

    Ok(PushArgsOwned { extra: None, args })
}
```

### Step 11: Workspace Integration UI

The workspace integration UI allows users to configure OAuth credentials for each service at the workspace level. This is in `frontend/src/lib/components/workspaceSettings/WorkspaceIntegrations.svelte`.

Add your service to the `supportedServices` map:

```typescript
const supportedServices: Record<string, ServiceConfig> = {
    // ... existing services ...
    newservice: {
        name: 'newservice',
        displayName: 'New Service',
        description: 'Connect to New Service for file triggers',
        icon: NewServiceIcon,
        docsUrl: 'https://www.windmill.dev/docs/integrations/newservice',
        // For self-hosted services that need a base URL:
        // requiresBaseUrl: true (default)
        // For cloud services with fixed OAuth endpoints:
        requiresBaseUrl: false,
        clientIdPlaceholder: 'Your OAuth client ID',
        clientSecretPlaceholder: 'Your OAuth client secret',
        setupInstructions: [
            'Step 1: Create an OAuth app on the service',
            'Step 2: Configure the redirect URI shown below',
            'Step 3: Enter the client credentials below'
        ]
    }
}
```

The `OAuthClientConfig.svelte` component handles the OAuth credential form. It accepts these props from the service config:

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `requiresBaseUrl` | `boolean` | `true` | Whether to show base URL field (false for cloud services like Google) |
| `clientIdPlaceholder` | `string` | auto | Placeholder text for client ID input |
| `clientSecretPlaceholder` | `string` | auto | Placeholder text for client secret input |
| `setupInstructions` | `string[]` | none | Step-by-step setup instructions (supports HTML) |

For services with `requiresBaseUrl: false`, the `base_url` field is sent as empty string `''` and the `isConfigured()` check skips the base_url validation.

### Step 12: Update `frontend/src/lib/components/triggers/utils.ts`

This file is the central registry for ALL trigger types (not just native triggers). You must update multiple maps/functions:

```typescript
// 1. Import icon
import NewServiceIcon from '$lib/components/icons/NewServiceIcon.svelte'

// 2. Add to triggerIconMap
export const triggerIconMap: Record<string, any> = {
    // ... existing entries ...
    newservice: NewServiceIcon,
}

// 3. Add to triggerDisplayNamesMap
export const triggerDisplayNamesMap: Record<string, string> = {
    // ... existing entries ...
    newservice: 'New Service',
}

// 4. Add to triggerTypeOrder in sortTriggers()
const triggerTypeOrder = [
    // ... existing entries ...
    'newservice',
]

// 5. Add to getLightConfig()
export function getLightConfig(trigger: any, triggerType: string) {
    // ... existing cases ...
    } else if (triggerType === 'newservice') {
        return {
            // Return key fields for display in trigger lists
            folder_path: trigger.service_config?.folderPath,
        }
    }
}

// 6. Add to getTriggerLabel()
export function getTriggerLabel(type: string, path: string, config: any) {
    // ... existing cases ...
    } else if (type === 'newservice' && path) {
        return `NewService: ${path}`
    }
}

// 7. Add to jobTriggerKinds array
export const jobTriggerKinds = [
    // ... existing entries ...
    'newservice',
] as const

// 8. Add to countPropertyMap
export const countPropertyMap: Record<string, string> = {
    // ... existing entries ...
    newservice: 'newservice_count',
}

// 9. Add to triggerSaveFunctions (if using NativeTriggerService)
export const triggerSaveFunctions: Record<string, Function> = {
    // ... existing entries ...
    newservice: NativeTriggerService.updateNewservice,
}
```

### Step 13: Update TriggersBadge Component

The `TriggersBadge.svelte` component (`frontend/src/lib/components/graph/renderers/triggers/TriggersBadge.svelte`) displays trigger icons with count badges in the script/flow editor. You must update three things:

1. **Import the icon** at the top of the file:
```typescript
import NewServiceIcon from '$lib/components/icons/NewServiceIcon.svelte'
```

2. **Add to `baseConfig`** inside `triggerTypeConfig` (this maps the trigger type to its icon and count key for the badge):
```typescript
const baseConfig = {
    // ... existing entries ...
    newservice: { icon: NewServiceIcon, countKey: 'newservice_count' }
}
```

3. **Add to the `allTypes` array** (this determines which trigger types can be displayed when no triggers are loaded yet):
```typescript
let allTypes = $derived([
    // ... existing entries ...
    'newservice'
])
```

> **Important:** The dynamic `availableNativeServices` loop (lines 76-79) adds services without a `countKey`, so it will NOT display count badges. You must add an explicit entry in `baseConfig` with the `countKey` for the count badge to appear.

### Step 14: Update OpenAPI Spec and Regenerate Types

In `backend/windmill-api/openapi.yaml`, add your service to the `JobTriggerKind` enum:

```yaml
JobTriggerKind:
  type: string
  enum:
    # ... existing values ...
    - newservice
```

Then regenerate the frontend TypeScript client:

```bash
cd frontend && npm run generate-backend-client
```

This ensures the generated `NativeServiceName` and `JobTriggerKind` types include your new service.

---

## Testing Checklist

- [ ] Database migration runs successfully
- [ ] `cargo sqlx prepare` regenerates query metadata
- [ ] Service appears in workspace integrations list
- [ ] OAuth flow completes successfully
- [ ] Can create a new trigger
- [ ] Can view trigger details
- [ ] Can update trigger configuration
- [ ] Can delete trigger
- [ ] Webhook receives and processes payloads
- [ ] Background sync detects deleted triggers
- [ ] Background sync updates config drift
- [ ] Error handling works (expired tokens, service unavailable)

---

## Reference Implementation

The Nextcloud implementation serves as the reference:

| File | Purpose |
|------|---------|
| `nextcloud/mod.rs` | Type definitions (OAuthData, ServiceConfig, TriggerData) |
| `nextcloud/external.rs` | External trait implementation |
| `nextcloud/routes.rs` | Additional routes (list events) |
| `NextcloudTriggerForm.svelte` | Frontend configuration form |
| `NextcloudIcon.svelte` | Service icon |

Study these files for patterns on:
- HTTP request handling with automatic token refresh
- Error handling for 404/401/403 responses
- Service config comparison for sync
- Dynamic schema generation in frontend
