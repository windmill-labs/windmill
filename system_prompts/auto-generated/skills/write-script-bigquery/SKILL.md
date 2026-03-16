---
name: write-script-bigquery
description: MUST use when writing BigQuery queries.
---

## CLI Commands

Place scripts in a folder. After writing, tell the user they can run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

Use `wmill resource-type list --schema` to discover available resource types.

# BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:

```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```
