# Resource Types

On Windmill, credentials and configuration are stored in resources. Resource types define the format of the resource.

## Using Resources in Scripts

### TypeScript (Bun/Deno)

Use the `RT` namespace for resource types as parameters:

```typescript
export async function main(stripe: RT.Stripe, db: RT.Postgresql) {
  // stripe and db contain the resource values
}
```

### Python

Redefine the resource type as a TypedDict (lowercase name):

```python
from typing import TypedDict

class postgresql(TypedDict):
    host: str
    port: int
    user: str
    password: str
    dbname: str

def main(db: postgresql):
    pass
```

### PHP

Define the resource type as a class:

```php
<?php

if (!class_exists('Postgresql')) {
    class Postgresql {
        public string $host;
        public int $port;
        public string $user;
        public string $password;
        public string $dbname;
    }
}

function main(Postgresql $db) {
    // ...
}
```

## Using Resources in Flows

### As Flow Input

In the flow schema, set the property type to `"object"` with format `"resource-{type}"`:

```json
{
  "type": "object",
  "properties": {
    "database": {
      "type": "object",
      "format": "resource-postgresql",
      "description": "Database connection"
    }
  }
}
```

### As Step Input (Static Reference)

Reference a specific resource using `$res:` prefix:

```json
{
  "database": {
    "type": "static",
    "value": "$res:f/folder/my_database"
  }
}
```

## Common Resource Types

- `postgresql` - PostgreSQL database connection
- `mysql` - MySQL database connection
- `mongodb` - MongoDB connection
- `s3` - S3-compatible storage
- `slack` - Slack API credentials
- `stripe` - Stripe API key
- `openai` - OpenAI API key
- `smtp` - Email server configuration
- `http` - Generic HTTP authentication
