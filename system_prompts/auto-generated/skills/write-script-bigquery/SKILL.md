---
name: write-script-bigquery
description: Write BigQuery queries with @name parameter syntax.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:

```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```
