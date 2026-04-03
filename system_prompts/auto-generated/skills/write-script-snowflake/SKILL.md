---
name: write-script-snowflake
description: MUST use when writing Snowflake queries.
---

## CLI Commands

Place scripts in a folder. After writing, tell the user they can run:
- `wmill generate-metadata <path_to_script_or_folder> --yes --skip-flows --skip-apps` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

Use `wmill resource-type list --schema` to discover available resource types.

# Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```
