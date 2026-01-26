---
name: resources
description: Manage resource types and credentials for external services.
---

# Windmill Resources

Resources store credentials and configuration for external services.

## File Format

Resource files use the pattern: `{path}.resource.json`

Example: `f/databases/postgres_prod.resource.json`

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

## Resource References

Reference other resources:

```json
{
  "value": {
    "database": "$res:f/databases/postgres"
  }
}
```

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

## CLI Commands

```bash
# List resources
wmill resource list

# List resource types with schemas
wmill resource-type list --schema

# Get specific resource type schema
wmill resource-type get postgresql

# Push resources
wmill sync push
```
