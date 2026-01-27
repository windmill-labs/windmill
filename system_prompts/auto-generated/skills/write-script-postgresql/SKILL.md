---
name: write-script-postgresql
description: Write PostgreSQL queries with $1::TYPE parameter syntax.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# PostgreSQL

Arguments are obtained directly in the statement with `$1::{type}`, `$2::{type}`, etc.

Name the parameters by adding comments at the beginning of the script (without specifying the type):

```sql
-- $1 name1
-- $2 name2 = default_value
SELECT * FROM users WHERE name = $1::TEXT AND age > $2::INT;
```
