# Microsoft SQL Server (MSSQL)

Arguments use `@P1`, `@P2`, etc.

Name the parameters by adding comments before the statement:

```sql
-- @P1 name1 (varchar)
-- @P2 name2 (int) = 0
SELECT * FROM users WHERE name = @P1 AND age > @P2;
```
