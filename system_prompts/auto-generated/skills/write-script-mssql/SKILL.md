---
name: write-script-mssql
description: MUST use when writing MS SQL Server queries.
---

## CLI Commands

Place scripts in a folder. After writing, tell the user they can run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

Use `wmill resource-type list --schema` to discover available resource types.

# Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:

```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```
