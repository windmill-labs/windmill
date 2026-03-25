## TypeScript Datatable API (windmill-client)

Import: `import * as wmill from 'windmill-client'`

SQL statement object with query content, arguments, and execution methods
```typescript
type SqlStatement<T> = {
  /** Raw SQL content with formatted arguments */
  content: string;

  /** Argument values keyed by parameter name */
  args: Record<string, any>;

  /**
  * Execute the SQL query and return results
  * @param params - Optional parameters including result collection mode
  * @returns Query results based on the result collection mode
  */
  fetch<ResultCollectionT extends ResultCollection = "last_statement_all_rows">(
  params?: FetchParams<ResultCollectionT | ResultCollection> // The union is for auto-completion
  ): Promise<SqlResult<T, ResultCollectionT>>;

  /**
  * Execute the SQL query and return only the first row
  * @param params - Optional parameters
  * @returns First row of the query result
  */
  fetchOne(
  params?: Omit<FetchParams<"last_statement_first_row">, "resultCollection">
  ): Promise<SqlResult<T, "last_statement_first_row">>;

  /**
  * Execute the SQL query and return only the first row as a scalar value
  * @param params - Optional parameters
  * @returns First row of the query result
  */
  fetchOneScalar(
  params?: Omit<
  FetchParams<"last_statement_first_row_scalar">,
  "resultCollection"
  >
  ): Promise<SqlResult<T, "last_statement_first_row_scalar">>;

  /**
  * Execute the SQL query without fetching rows
  * @param params - Optional parameters
  */
  execute(
  params?: Omit<FetchParams<"last_statement_first_row">, "resultCollection">
  ): Promise<void>;
};
```

```typescript
// Template tag function: sql`SELECT * FROM table WHERE id = ${id}`.fetch()
interface DatatableSqlTemplateFunction {
  // Tagged template usage:
  <T = any>(strings: TemplateStringsArray, ...values: any[]): SqlStatement<T>;
  query<T = any>(sql: string, ...params: any[]): SqlStatement<T>;
};
```

Create a SQL template function for PostgreSQL/datatable queries
@param name - Database/datatable name (default: "main")
@returns SQL template function for building parameterized queries
@example
let sql = wmill.datatable()
let name = 'Robin'
let age = 21
await sql`
  SELECT * FROM friends
    WHERE name = ${name} AND age = ${age}::int
`.fetch()
```typescript
function datatable(name: string = "main"): DatatableSqlTemplateFunction
```
