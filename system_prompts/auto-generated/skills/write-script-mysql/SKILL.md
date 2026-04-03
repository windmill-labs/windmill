---
name: write-script-mysql
description: MUST use when writing MySQL queries.
---

## CLI Commands

Place scripts in a folder. After writing, tell the user they can run:
- `wmill generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

Use `wmill resource-type list --schema` to discover available resource types.

# MySQL

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (int) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```
