# Windmill Resources

Resources store credentials and configuration for external services.

<!-- cli-only -->
## File Format

Resource files use the pattern: `{path}.resource.json`

Example: `f/databases/postgres_prod.resource.json`
<!-- /cli-only -->

## Resource Structure

```json
{
  "value": {
    "host": "db.example.com",
    "port": 5432,
    "user": "admin",
    "password": "$var:g/all/db_password",
    "dbname": "production"
  },
  "description": "Production PostgreSQL database",
  "resource_type": "postgresql"
}
```

## Required Fields

- `value` - Object containing the resource configuration
- `resource_type` - Name of the resource type (e.g., "postgresql", "slack")

## Variable References

Reference variables in resource values:

```json
{
  "value": {
    "api_key": "$var:g/all/api_key",
    "secret": "$var:u/admin/secret"
  }
}
```

**Reference formats:**
- `$var:g/all/name` - Global variable
- `$var:u/username/name` - User variable
- `$var:f/folder/name` - Folder variable

## Secrets

Never put a secret (password, API key, token) inline in a resource value. Store it in a secret variable and reference that variable as `$var:<path>`.

- `$var:<path>` is a reference, not a value: it resolves to the variable's value at run time. Never invent a value for a variable.
- A resource that references a variable needs the variable to exist first, so create or deploy the variable before the resource.
<!-- cli-only -->
- A secret's plaintext never goes in a file of the repo: a `.variable.yaml` holding it would be committed. Ask the user to create the secret on the workspace instead, e.g. `wmill variable add '<value>' <path>` (a secret by default), then reference it by path.
<!-- /cli-only -->

## Resource References

Reference other resources:

```json
{
  "value": {
    "database": "$res:f/databases/postgres"
  }
}
```

## Passing a Resource or Variable as a Run Argument

A script or flow argument typed as a resource (schema `format: resource-<type>`) is passed as
the **bare string** `$res:<path>` — the whole argument value. Same for a variable, with
`$var:<path>`. This applies everywhere job arguments are supplied: `wmill script run/preview`,
`wmill flow run/preview`, the `runScriptByPath` / `runFlowByPath` API, a schedule's `args`, a
trigger's configured static args.

```json
{
  "db": "$res:f/databases/postgres_prod",
  "api_token": "$var:g/all/api_token"
}
```

The reference is resolved when the job runs, under the job's run-as identity — the caller for an
ordinary run, but the configured principal for a schedule, a trigger, or a runnable set to run on
behalf of someone else. The run fails if that identity cannot read the referenced resource or
variable.

**Never wrap it in an object.** The resolver only rewrites a JSON value that *is* a string
starting with `$res:` / `$var:`; keys are never inspected. These are all wrong and are passed
through to the script unchanged:

```json
{ "db": { "$res": "f/databases/postgres_prod" } }
{ "db": { "resource": "f/databases/postgres_prod" } }
{ "db": "f/databases/postgres_prod" }
```

The string may sit anywhere a string can — a top-level argument, a nested object field
(`{ "gh_auth": { "token": "$var:g/all/gh_token" } }`), or an array element (array elements are
walked only while nested at most two levels deep, and only for arrays of at most 1000 items).
The prefix must be on the string itself.

## Common Resource Types

### PostgreSQL
```json
{
  "resource_type": "postgresql",
  "value": {
    "host": "localhost",
    "port": 5432,
    "user": "postgres",
    "password": "$var:g/all/pg_password",
    "dbname": "windmill",
    "sslmode": "prefer"
  }
}
```

### MySQL
```json
{
  "resource_type": "mysql",
  "value": {
    "host": "localhost",
    "port": 3306,
    "user": "root",
    "password": "$var:g/all/mysql_password",
    "database": "myapp"
  }
}
```

### Slack
```json
{
  "resource_type": "slack",
  "value": {
    "token": "$var:g/all/slack_token"
  }
}
```

### AWS S3
```json
{
  "resource_type": "s3",
  "value": {
    "bucket": "my-bucket",
    "region": "us-east-1",
    "accessKeyId": "$var:g/all/aws_access_key",
    "secretAccessKey": "$var:g/all/aws_secret_key"
  }
}
```

### HTTP/API
```json
{
  "resource_type": "http",
  "value": {
    "baseUrl": "https://api.example.com",
    "headers": {
      "Authorization": "Bearer $var:g/all/api_token"
    }
  }
}
```

### Kafka
```json
{
  "resource_type": "kafka",
  "value": {
    "brokers": "broker1:9092,broker2:9092",
    "sasl_mechanism": "PLAIN",
    "security_protocol": "SASL_SSL",
    "username": "$var:g/all/kafka_user",
    "password": "$var:g/all/kafka_password"
  }
}
```

### NATS
```json
{
  "resource_type": "nats",
  "value": {
    "servers": ["nats://localhost:4222"],
    "user": "$var:g/all/nats_user",
    "password": "$var:g/all/nats_password"
  }
}
```

### MQTT
```json
{
  "resource_type": "mqtt",
  "value": {
    "host": "mqtt.example.com",
    "port": 8883,
    "username": "$var:g/all/mqtt_user",
    "password": "$var:g/all/mqtt_password",
    "tls": true
  }
}
```

## When No Resource Type Fits

The Windmill Hub publishes resource types for most services, and an instance gets them by syncing with the hub. When none fits the service:

- If looking up resource types reports that the instance is missing the hub's types, mention that a superadmin can sync them: the hub most likely has one for the service. Create one of your own only if the user still wants it, since after a sync the workspace would hold both.
- Otherwise, or when the user prefers it to syncing, propose a resource type holding the service's connection fields (base URL, account or region identifiers, credentials), as in "Custom Resource Types" below, and ask the user before creating it: every resource of that type and every script taking one depends on its fields.
<!-- chat-only -->
- You cannot create a resource type yourself: give the user the schema to add with **Add resource type** on the Resources page.
<!-- /chat-only -->
- Use a plain variable instead of a resource only when the user prefers it.

## Custom Resource Types

Create custom resource types with JSON Schema:

```json
{
  "name": "custom_api",
  "schema": {
    "type": "object",
    "properties": {
      "base_url": {"type": "string", "format": "uri"},
      "api_key": {"type": "string"},
      "timeout": {"type": "integer", "default": 30}
    },
    "required": ["base_url", "api_key"]
  },
  "description": "Custom API connection"
}
```

Save as: `custom_api.resource-type.json`

## OAuth Resources

OAuth resources are managed through the Windmill UI and marked:

```json
{
  "is_oauth": true,
  "account": 123
}
```

OAuth tokens are automatically refreshed by Windmill.

## Using Resources in Scripts

### TypeScript (Bun/Deno)
```typescript
export async function main(db: RT.Postgresql) {
  // db contains the resource values
  const { host, port, user, password, dbname } = db;
}
```

### Python
```python
class postgresql(TypedDict):
    host: str
    port: int
    user: str
    password: str
    dbname: str

def main(db: postgresql):
    # db contains the resource values
    pass
```

<!-- cli-only -->
## CLI Commands

```bash
# List resources
wmill resource list

# List resource types with schemas
wmill resource-type list --schema

# Get specific resource type schema
wmill resource-type get postgresql

# Deploy resources to the workspace — destructive to remote state, so only run when
# the user explicitly asks to deploy/publish/push. Depending on how the repo is wired,
# deploy via `git push` or `wmill sync push` (see the Deploying section in AGENTS.wmill.md).
wmill sync push
```
<!-- /cli-only -->
