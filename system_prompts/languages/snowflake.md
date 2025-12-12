# Snowflake

Arguments use `?` placeholders.

Name the parameters by adding comments before the statement:

```sql
-- ? name1 (text)
-- ? name2 (number) = 0
SELECT * FROM users WHERE name = ? AND age > ?;
```
