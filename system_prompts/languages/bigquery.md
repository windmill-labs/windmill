# BigQuery

Arguments use `@name` syntax.

Name the parameters by adding comments before the statement:

```sql
-- @name1 (string)
-- @name2 (int64) = 0
SELECT * FROM users WHERE name = @name1 AND age > @name2;
```
