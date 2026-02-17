# Windmill Instance Configuration as Code

Windmill supports managing instance configuration (global settings + worker group configs) declaratively through YAML files. This enables Infrastructure-as-Code (IaC) workflows where your Windmill instance settings are version-controlled and applied automatically.

Two deployment models are supported:

| Approach | Best for | Requires |
|---|---|---|
| **`sync-config`** CLI | Docker Compose, VMs, CI/CD pipelines | Database access |
| **Kubernetes Operator** | Kubernetes clusters | `operator` feature flag, RBAC |

Both use the same YAML schema (`InstanceConfig`) and the same secret reference mechanisms.

---

## Config File Reference

A Windmill instance config file has two top-level keys:

```yaml
global_settings:
  # Instance-wide settings (stored in the global_settings table)
  base_url: "https://windmill.example.com"
  retention_period_secs: 2592000
  # ...

worker_configs:
  # Worker group configurations (stored in the config table as worker__<name>)
  default:
    worker_tags: ["deno", "python3", "bun", "go", "bash"]
  gpu:
    dedicated_worker: "ws:f/gpu_inference"
    # ...
```

All fields are optional. Only the fields you specify are synced to the database.

### Sensitive Field References

Fields that contain secrets (license keys, OAuth secrets, SMTP passwords, etc.) support three formats:

```yaml
# 1. Plain literal (not recommended for production)
license_key: "my-license-key"

# 2. Environment variable reference (works everywhere)
license_key:
  envRef: "WM_LICENSE_KEY"

# 3. Kubernetes Secret reference (K8s only)
license_key:
  secretKeyRef:
    name: windmill-secrets    # Secret resource name
    key: license-key          # Key within the Secret
```

Fields that support `envRef` and `secretKeyRef`:

- `license_key`
- `hub_api_secret`
- `scim_token`
- `smtp_settings.smtp_password`
- `oauths.<provider>.secret` (each OAuth client secret)
- `custom_instance_pg_databases.user_pwd`

---

## Docker Compose (`sync-config`)

The `sync-config` subcommand reads a YAML config file, resolves any `envRef` references from the process environment, and syncs the result to the database.

### How it works

1. Windmill reads and parses the YAML file
2. Any `envRef` fields are resolved from the container's environment variables
3. The current database state is read
4. A diff is computed (using `Replace` mode: settings absent from the file are deleted, except protected ones like `ducklake_settings`)
5. Changes are applied to the database

### Setup

See the included [`docker-compose.yml`](docker-compose.yml) for a complete working example. The key parts:

**1. Create your config file** (`windmill-config.yaml`):

```yaml
global_settings:
  base_url: "https://windmill.example.com"
  license_key:
    envRef: "WM_LICENSE_KEY"
  retention_period_secs: 2592000
  expose_metrics: true
  smtp_settings:
    smtp_host: "smtp.example.com"
    smtp_port: 587
    smtp_from: "windmill@example.com"
    smtp_password:
      envRef: "SMTP_PASSWORD"
  oauths:
    google:
      id: "google-client-id"
      secret:
        envRef: "GOOGLE_OAUTH_SECRET"
      login_config:
        auth_url: "https://accounts.google.com/o/oauth2/v2/auth"
        token_url: "https://oauth2.googleapis.com/token"
        userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo"
        scopes: ["openid", "profile", "email"]
  custom_tags:
    - gpu
    - high-mem

worker_configs:
  default:
    worker_tags: ["deno", "python3", "bun", "go", "bash", "powershell"]
    init_bash: "echo 'Worker starting'"
  native:
    worker_tags: ["nativets"]
```

**2. Add an init container to `docker-compose.yml`**:

```yaml
services:
  windmill_config_sync:
    image: ${WM_IMAGE}
    # Run once at startup then exit
    restart: "no"
    command: ["windmill", "sync-config", "/config/windmill-config.yaml"]
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - WM_LICENSE_KEY=${WM_LICENSE_KEY}
      - SMTP_PASSWORD=${SMTP_PASSWORD}
      - GOOGLE_OAUTH_SECRET=${GOOGLE_OAUTH_SECRET}
    volumes:
      - ./windmill-config.yaml:/config/windmill-config.yaml:ro
    depends_on:
      db:
        condition: service_healthy
```

**3. Set secrets in your `.env` file** (not committed to version control):

```env
DATABASE_URL=postgres://postgres:changeme@db/windmill
WM_IMAGE=ghcr.io/windmill-labs/windmill-ee:main
WM_LICENSE_KEY=your-license-key-here
SMTP_PASSWORD=your-smtp-password
GOOGLE_OAUTH_SECRET=your-google-oauth-secret
```

### Re-syncing after config changes

The `sync-config` container runs once and exits. To re-apply after editing the YAML:

```bash
docker compose run --rm windmill_config_sync
```

Or, for CI/CD pipelines, run the binary directly:

```bash
windmill sync-config ./windmill-config.yaml
```

### Replace semantics

`sync-config` uses **Replace** mode: any global setting present in the database but absent from your YAML file will be **deleted** (except protected settings like `ducklake_settings` and `custom_instance_pg_databases`). This ensures the database state matches the file exactly.

If you only want to manage a subset of settings, include all settings you want to keep in the YAML file.

---

## Kubernetes (Operator)

The Windmill Kubernetes operator watches a ConfigMap and continuously reconciles the database to match the declared state. It also supports `secretKeyRef` to pull values from Kubernetes Secrets natively.

### Prerequisites

- Windmill built with the `operator` feature flag
- RBAC permissions for the operator pod (see below)
- A ConfigMap named `windmill-instance` (or a custom name via the `OPERATOR_CONFIGMAP` env var)

### Setup

**1. Create a Kubernetes Secret for sensitive values**:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: windmill-secrets
  namespace: windmill
type: Opaque
stringData:
  license-key: "your-license-key-here"
  smtp-password: "your-smtp-password"
  google-oauth-secret: "your-google-oauth-secret"
```

**2. Create the ConfigMap** (`windmill-instance.yaml`):

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: windmill-instance
  namespace: windmill
data:
  spec: |
    global_settings:
      base_url: "https://windmill.example.com"
      license_key:
        secretKeyRef:
          name: windmill-secrets
          key: license-key
      retention_period_secs: 2592000
      expose_metrics: true
      smtp_settings:
        smtp_host: "smtp.example.com"
        smtp_port: 587
        smtp_from: "windmill@example.com"
        smtp_password:
          secretKeyRef:
            name: windmill-secrets
            key: smtp-password
      oauths:
        google:
          id: "google-client-id"
          secret:
            secretKeyRef:
              name: windmill-secrets
              key: google-oauth-secret
          login_config:
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth"
            token_url: "https://oauth2.googleapis.com/token"
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo"
            scopes: ["openid", "profile", "email"]
      custom_tags:
        - gpu
        - high-mem

    worker_configs:
      default:
        worker_tags: ["deno", "python3", "bun", "go", "bash", "powershell"]
        init_bash: "echo 'Worker starting'"
      native:
        worker_tags: ["nativets"]
```

The config lives under `data.spec` as a YAML string. This is the same schema used by `sync-config`.

**3. Apply**:

```bash
kubectl apply -f windmill-instance.yaml
```

**4. Check sync status** via events:

```bash
kubectl get events --field-selector involvedObject.name=windmill-instance
```

### License key handling

If `license_key` is absent or empty in the ConfigMap but already exists in the database, the operator preserves the database value. This lets you manage the license key separately (e.g., via the UI) without the operator overwriting it.

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `OPERATOR_NAMESPACE` | Pod's own namespace | Namespace of the ConfigMap |
| `OPERATOR_CONFIGMAP` | `windmill-instance` | Name of the ConfigMap to watch |

### Using `envRef` in Kubernetes

`envRef` also works in the operator context. Values are resolved from the operator pod's environment. This is useful when secrets are injected via pod env vars (e.g., from a vault sidecar):

```yaml
data:
  spec: |
    global_settings:
      license_key:
        envRef: "WM_LICENSE_KEY"   # Read from operator pod env
```

The operator pod's Deployment would include:

```yaml
env:
  - name: WM_LICENSE_KEY
    valueFrom:
      secretKeyRef:
        name: windmill-secrets
        key: license-key
```

This is functionally equivalent to using `secretKeyRef` directly in the ConfigMap, but lets you use any secret injection mechanism your cluster supports (external-secrets, vault-agent, etc.).

### RBAC

The operator pod needs permissions to read ConfigMaps, Secrets, and create Events. Minimal Role:

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: windmill-operator
  namespace: windmill
rules:
  - apiGroups: [""]
    resources: ["configmaps"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["secrets"]
    verbs: ["get", "list", "watch"]
  - apiGroups: [""]
    resources: ["events"]
    verbs: ["create", "patch"]
```

Note: This is now a namespace-scoped **Role** (not ClusterRole), since there is no CRD to manage.

### Running the operator

```bash
# As a standalone process (for development)
DATABASE_URL=postgres://... windmill operator

# In production, deploy as a Kubernetes Deployment
```

---

## Choosing Between `envRef` and `secretKeyRef`

| Feature | `envRef` | `secretKeyRef` |
|---|---|---|
| Works in Docker Compose | Yes | No |
| Works in Kubernetes | Yes | Yes |
| Works with vault sidecars | Yes | No (use `envRef` instead) |
| Reads from | Process environment | K8s Secrets API |
| Requires RBAC for Secrets | No | Yes |

**Recommendation**: Use `envRef` for portability across deployment targets. Use `secretKeyRef` when you want direct Kubernetes-native secret binding without intermediate env vars.
