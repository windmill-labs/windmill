import { getEnv, getWorkspace, workerHasInternalServer } from "./client";
import { OpenAPI } from "./core/OpenAPI";
import { JobService } from "./services.gen";

type ResultCollection =
  | "last_statement_all_rows"
  | "last_statement_first_row"
  | "last_statement_all_rows_scalar"
  | "last_statement_first_row_scalar"
  | "all_statements_all_rows"
  | "all_statements_first_row"
  | "all_statements_all_rows_scalar"
  | "all_statements_first_row_scalar"
  | "legacy";

type FetchParams<ResultCollectionT extends ResultCollection> = {
  resultCollection?: ResultCollectionT;
};

type SqlResult<
  T,
  ResultCollectionT extends ResultCollection
> = ResultCollectionT extends "last_statement_first_row"
  ? T | null
  : ResultCollectionT extends "all_statements_first_row"
  ? T[]
  : ResultCollectionT extends "last_statement_all_rows"
  ? T[]
  : ResultCollectionT extends "all_statements_all_rows"
  ? T[][]
  : ResultCollectionT extends "last_statement_all_rows_scalar"
  ? T[keyof T][]
  : ResultCollectionT extends "all_statements_all_rows_scalar"
  ? T[keyof T][][]
  : ResultCollectionT extends "last_statement_first_row_scalar"
  ? T[keyof T] | null
  : ResultCollectionT extends "all_statements_first_row_scalar"
  ? T[keyof T][]
  : unknown;
/**
 * SQL statement object with query content, arguments, and execution methods
 */
export type SqlStatement<T> = {
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

/**
 * Wrapper for raw SQL fragments that should be inlined without parameterization.
 * Created via `sql.raw(value)`.
 */
export class RawSql {
  readonly __brand = "RawSql" as const;
  constructor(public readonly value: string) { }
}

/**
 * Template tag function for creating SQL statements with parameterized values
 */
export interface SqlTemplateFunction {
  <T = any>(strings: TemplateStringsArray, ...values: any[]): SqlStatement<T>;
  /** Create a raw SQL fragment that will be inlined without parameterization */
  raw(value: string): RawSql;
}

export interface DatatableSqlTemplateFunction extends SqlTemplateFunction {
  query<T = any>(sql: string, ...params: any[]): SqlStatement<T>;
}

// ---------------------------------------------------------------------------
// Provider interface — captures what differs between datatable and ducklake
// ---------------------------------------------------------------------------

interface SqlProvider {
  formatArgDecl(argNum: number, argType: string): string;
  formatArgUsage(
    argNum: number,
    explicitType: string | undefined,
    inferredType: string
  ): string;
  preamble(): string;
  language: "postgresql" | "duckdb";
  extraArgs: Record<string, any>;
  providerName: string;
}

function datatableProvider(name: string, schema?: string): SqlProvider {
  return {
    providerName: "datatable",
    language: "postgresql",
    extraArgs: { database: `datatable://${name}` },
    formatArgDecl: (argNum) => `-- $${argNum} arg${argNum}`,
    formatArgUsage: (argNum, explicitType, inferredType) =>
      explicitType !== undefined
        ? `$${argNum}`
        : `$${argNum}::${inferredType}`,
    preamble: () => (schema ? `SET search_path TO "${schema}";\n` : ""),
  };
}

function ducklakeProvider(name: string, schema?: string): SqlProvider {
  return {
    providerName: "ducklake",
    language: "duckdb",
    extraArgs: {},
    formatArgDecl: (argNum, argType) => `-- $arg${argNum} (${argType})`,
    formatArgUsage: (argNum) => `$arg${argNum}`,
    // `USE dl."schema"` sets the active schema so unqualified tables resolve there.
    preamble: () =>
      `ATTACH 'ducklake://${name}' AS dl;USE dl${schema ? `."${schema}"` : ""};\n`,
  };
}

// ---------------------------------------------------------------------------
// Shared template function builder
// ---------------------------------------------------------------------------

// Build a ready-to-execute SqlStatement. Used by both the template-tag
// path (which builds `content` from strings/values) and `.query()` (which
// gets a hand-written SQL string with positional placeholders).
function buildSqlStatement(
  provider: SqlProvider,
  content: string,
  contentBody: string,
  args: Record<string, any>
): SqlStatement<any> {
  async function fetch<ResultCollectionT extends ResultCollection>({
    resultCollection,
  }: FetchParams<ResultCollectionT> = {}) {
    let finalContent = content;
    if (resultCollection)
      finalContent = `-- result_collection=${resultCollection}\n${finalContent}`;
    try {
      let result;
      if (workerHasInternalServer()) {
        result = await JobService.runScriptPreviewInline({
          workspace: getWorkspace(),
          requestBody: { args, content: finalContent, language: provider.language },
        });
      } else {
        result = await JobService.runScriptPreviewAndWaitResult({
          workspace: getWorkspace(),
          requestBody: { args, content: finalContent, language: provider.language },
        });
      }
      return result as SqlResult<any, ResultCollectionT>;
    } catch (e: any) {
      let err = e;
      if (
        e &&
        typeof e.body == "string" &&
        e.statusText == "Internal Server Error"
      ) {
        let body = e.body;
        if (body.startsWith("Internal:")) body = body.slice(9).trim();
        if (body.startsWith("Error:")) body = body.slice(6).trim();
        if (body.startsWith("datatable")) body = body.slice(9).trim();
        err = Error(`${provider.providerName} ${body}`);
        err.query = contentBody;
        err.request = e.request;
      }
      throw err;
    }
  }

  return {
    content,
    args,
    fetch,
    fetchOne: (params) =>
      fetch({ ...params, resultCollection: "last_statement_first_row" }),
    fetchOneScalar: (params) =>
      fetch({
        ...params,
        resultCollection: "last_statement_first_row_scalar",
      }),
    execute: (params) => fetch(params),
  } satisfies SqlStatement<any>;
}

// JSON-encode a JS value into something the executor can deserialize. The
// JSON.stringify-friendly representation of a JS value before sending it to
// the executor:
//   - `bigint`             → string. JSON.stringify on a bigint throws; the
//                            executor accepts numeric strings into BIGINT
//                            slots via `Value::String → INT8`.
//   - `Date`               → ISO-8601 string. inferSqlType maps these to
//                            `TIMESTAMPTZ`; the executor's `Value::String`
//                            arm parses ISO strings into `chrono::DateTime`.
//   - non-finite `number`  → string ("NaN" / "Infinity" / "-Infinity").
//                            JSON.stringify renders these as `null`, which
//                            silently became NULL in the database. The
//                            executor accepts these literals via
//                            `Value::String → FLOAT8` (`f64::from_str`).
//   - everything else      → passed through unchanged.
function serializeArgValue(v: any): any {
  if (typeof v === "bigint") return v.toString();
  if (v instanceof Date) return v.toISOString();
  if (typeof v === "number" && !Number.isFinite(v)) {
    if (Number.isNaN(v)) return "NaN";
    return v > 0 ? "Infinity" : "-Infinity";
  }
  return v;
}

function buildSqlTemplateFunction(provider: SqlProvider): SqlTemplateFunction {
  let sqlFn = ((strings: TemplateStringsArray, ...values: any[]) => {
    // Separate raw vs parameterized values, assigning arg indices only to params
    let argIndex = 0;
    const valueInfos = values.map((v, i) => {
      if (v instanceof RawSql)
        return { raw: true as const, value: v.value, originalIndex: i };
      argIndex++;
      return {
        raw: false as const,
        value: v,
        originalIndex: i,
        argNum: argIndex,
      };
    });

    // Arg declarations (SQL comments consumed by the executor)
    let argDecls = valueInfos
      .filter((info): info is Extract<(typeof valueInfos)[number], { raw: false }> => !info.raw)
      .map((info) => {
        let argType =
          parseTypeAnnotation(
            strings[info.originalIndex],
            strings[info.originalIndex + 1]
          ) || inferSqlType(info.value);
        return provider.formatArgDecl(info.argNum, argType);
      });

    let content = argDecls.length ? argDecls.join("\n") + "\n" : "";
    content += provider.preamble();

    // SQL body — inline raw values, reference params via provider syntax
    let contentBody = "";
    for (let i = 0; i < strings.length; i++) {
      contentBody += strings[i];
      if (i < valueInfos.length) {
        let info = valueInfos[i];
        if (info.raw) {
          contentBody += info.value;
        } else {
          let explicitType = parseTypeAnnotation(
            strings[info.originalIndex],
            strings[info.originalIndex + 1]
          );
          let inferredType = inferSqlType(info.value);
          contentBody += provider.formatArgUsage(
            info.argNum,
            explicitType,
            inferredType
          );
        }
      }
    }
    content += contentBody;

    const args = {
      ...Object.fromEntries(
        valueInfos
          .filter((info): info is Extract<(typeof valueInfos)[number], { raw: false }> => !info.raw)
          .map((info) => [`arg${info.argNum}`, serializeArgValue(info.value)])
      ),
      ...provider.extraArgs,
    };

    return buildSqlStatement(provider, content, contentBody, args);
  }) as SqlTemplateFunction;

  sqlFn.raw = (value: string) => new RawSql(value);
  return sqlFn;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Create a SQL template function for PostgreSQL/datatable queries
 * @param name - Database/datatable name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
export function datatable(name: string = "main"): DatatableSqlTemplateFunction {
  let { name: n, schema } = parseName(name);
  let provider = datatableProvider(n, schema);
  let sqlFn = buildSqlTemplateFunction(provider) as DatatableSqlTemplateFunction;
  // `.query(sql, ...params)` is for SQL strings that already contain
  // positional placeholders ($1, $2, ...). We DON'T go through the template
  // builder here — that would re-emit each value as `$N::TYPE` and append
  // them after the user's literal SQL, which is the bug previous versions of
  // this method shipped. Instead we build the executor-shaped content
  // directly: a `-- $N argN (TYPE)` declaration block (the parser picks
  // these up as explicitly typed args) followed by the user's SQL verbatim.
  // Note: we hand-roll the decl format here rather than calling
  // `provider.formatArgDecl`, because the datatable formatter intentionally
  // omits the type (the template-tag path emits `$N::TYPE` inline instead);
  // for `.query()` we have no inline cast to fall back on.
  sqlFn.query = (sqlString: string, ...params: any[]) => {
    let argDecls = params
      .map((v, i) => `-- $${i + 1} arg${i + 1} (${inferSqlType(v)})`)
      .join("\n");
    let contentBody = sqlString;
    let content =
      (argDecls ? argDecls + "\n" : "") + provider.preamble() + sqlString;
    let args = {
      ...Object.fromEntries(
        params.map((v, i) => [`arg${i + 1}`, serializeArgValue(v)])
      ),
      ...provider.extraArgs,
    };
    return buildSqlStatement(provider, content, contentBody, args);
  };
  return sqlFn;
}

/**
 * Create a SQL template function for DuckDB/ducklake queries
 * @param name - DuckDB database name, optionally with a schema as `name:schema` (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 * @example
 * // Target a specific schema within the ducklake
 * let sql = wmill.ducklake("my_lake:analytics")
 */
export function ducklake(name: string = "main"): SqlTemplateFunction {
  let { name: n, schema } = parseName(name);
  return buildSqlTemplateFunction(ducklakeProvider(n, schema));
}

/** Options for the ducklake materialization helpers. `partition` is bound as a
 * DuckDB arg (never interpolated); `selectSql`, `table`, `schema`, `uniqueKey`
 * are trusted structural SQL inlined via `raw`. */
export interface DucklakeMaterializeOptions {
  /** ducklake name (default "main"), optionally "name:schema". */
  ducklake?: string;
  /** target table within the ducklake. */
  table: string;
  /** the SELECT producing the rows for this slice. */
  selectSql: string;
  /** the partition value (bound). Omit for a whole-table materialization — no
   * partition column, and replace becomes a `CREATE OR REPLACE TABLE`. */
  partition?: string;
  /** dedup key → upsert in slice (delete-by-key + insert); omit → replace (delete partition + insert). */
  uniqueKey?: string;
  /** physical partition column (default "_wm_partition"). */
  partitionCol?: string;
}

/** Idempotently materialize `selectSql` into a ducklake table for one
 * partition (or the whole table when `partition` is omitted) — the client-side
 * equivalent of the `// materialize` engine.
 * With `uniqueKey` it upserts the slice (delete-by-key + insert); otherwise it
 * replaces it (whole table → `CREATE OR REPLACE`; partition → delete + insert).
 * Safe to re-run for the same partition (backfill / failure-recovery).
 *
 * Returns a lazy statement — call `.execute()` to run it:
 * `await wmill.upsertPartition({ table, selectSql, partition }).execute()`. */
export function upsertPartition(opts: DucklakeMaterializeOptions): SqlStatement<any> {
  return finishMaterialize(buildUpsertStatement(opts), opts);
}
function buildUpsertStatement(opts: DucklakeMaterializeOptions): SqlStatement<any> {
  let { name: n, schema } = parseName(opts.ducklake ?? "main");
  let sql = buildSqlTemplateFunction(ducklakeProvider(n, schema));
  let pcol = sql.raw(opts.partitionCol ?? "_wm_partition");
  let t = sql.raw(`dl.${opts.table}`);
  let body = sql.raw(opts.selectSql);
  // Whole-table (no partition): no partition column. Replace rebuilds the table
  // with CREATE OR REPLACE (handles schema changes); merge upserts by key.
  if (opts.partition === undefined) {
    if (opts.uniqueKey) {
      let uk = sql.raw(opts.uniqueKey);
      return sql`CREATE TABLE IF NOT EXISTS ${t} AS SELECT * FROM (${body}) WHERE false;
BEGIN TRANSACTION;
DELETE FROM ${t} WHERE ${uk} IN (SELECT ${uk} FROM (${body}));
INSERT INTO ${t} SELECT * FROM (${body});
COMMIT;`;
    }
    return sql`CREATE OR REPLACE TABLE ${t} AS SELECT * FROM (${body});`;
  }
  if (opts.uniqueKey) {
    let uk = sql.raw(opts.uniqueKey);
    // Upsert via delete-by-key + insert (not MERGE — DuckLake's MERGE fails
    // writing the first rows of a fresh partition).
    return sql`CREATE TABLE IF NOT EXISTS ${t} AS SELECT *, CAST(NULL AS VARCHAR) AS ${pcol} FROM (${body}) WHERE false;
ALTER TABLE ${t} SET PARTITIONED BY (${pcol});
BEGIN TRANSACTION;
DELETE FROM ${t} WHERE ${pcol} = ${opts.partition} AND ${uk} IN (SELECT ${uk} FROM (${body}));
INSERT INTO ${t} SELECT *, ${opts.partition} AS ${pcol} FROM (${body});
COMMIT;`;
  }
  return sql`CREATE TABLE IF NOT EXISTS ${t} AS SELECT *, CAST(NULL AS VARCHAR) AS ${pcol} FROM (${body}) WHERE false;
ALTER TABLE ${t} SET PARTITIONED BY (${pcol});
BEGIN TRANSACTION;
DELETE FROM ${t} WHERE ${pcol} = ${opts.partition};
INSERT INTO ${t} SELECT *, ${opts.partition} AS ${pcol} FROM (${body});
COMMIT;`;
}

/** INSERT-only materialization (no dedup/replace) for append-only tables.
 * Re-running the same partition duplicates rows — use only for immutable
 * event-log sources.
 *
 * Returns a lazy statement — call `.execute()` to run it:
 * `await wmill.appendPartition({ table, selectSql, partition }).execute()`. */
export function appendPartition(
  opts: Omit<DucklakeMaterializeOptions, "uniqueKey">,
): SqlStatement<any> {
  return finishMaterialize(buildAppendStatement(opts), opts);
}
function buildAppendStatement(
  opts: Omit<DucklakeMaterializeOptions, "uniqueKey">,
): SqlStatement<any> {
  let { name: n, schema } = parseName(opts.ducklake ?? "main");
  let sql = buildSqlTemplateFunction(ducklakeProvider(n, schema));
  let pcol = sql.raw(opts.partitionCol ?? "_wm_partition");
  let t = sql.raw(`dl.${opts.table}`);
  let body = sql.raw(opts.selectSql);
  // Whole-table (no partition): insert into the bare table, no partition column.
  if (opts.partition === undefined) {
    return sql`CREATE TABLE IF NOT EXISTS ${t} AS SELECT * FROM (${body}) WHERE false;
INSERT INTO ${t} SELECT * FROM (${body});`;
  }
  return sql`CREATE TABLE IF NOT EXISTS ${t} AS SELECT *, CAST(NULL AS VARCHAR) AS ${pcol} FROM (${body}) WHERE false;
ALTER TABLE ${t} SET PARTITIONED BY (${pcol});
INSERT INTO ${t} SELECT *, ${opts.partition} AS ${pcol} FROM (${body});`;
}

// In pipeline context (WM_PIPELINE), wrap a materialize statement so a
// successful run captures the slice's row count + snapshot and records
// materialized_partition state — making SDK materializations appear in the grid
// like `// materialize` ones. Outside a pipeline it's a passthrough (no record).
function finishMaterialize(
  stmt: SqlStatement<any>,
  opts: Pick<DucklakeMaterializeOptions, "ducklake" | "table" | "partition" | "partitionCol">,
): SqlStatement<any> {
  if (getEnv("WM_PIPELINE") !== "true") return stmt;
  let { name: n, schema } = parseName(opts.ducklake ?? "main");
  let sql = buildSqlTemplateFunction(ducklakeProvider(n, schema));
  let t = sql.raw(`dl.${opts.table}`);
  let pcol = opts.partitionCol ?? "_wm_partition";
  let where =
    opts.partition !== undefined
      ? sql.raw(`WHERE ${pcol} = '${String(opts.partition).replace(/'/g, "''")}'`)
      : sql.raw("");
  let summary = sql`SELECT (SELECT count(*) FROM ${t} ${where}) AS rows, (SELECT max(snapshot_id) FROM ducklake_snapshots('dl')) AS snapshot_id`;
  // Asset path mirrors the `// materialize` engine: <lake>/<schema>.<table> for
  // an explicit schema, else <lake>/<table> — so the grid lookup matches and
  // distinct schemas don't collide under one state key.
  let assetPath = schema ? `${n}/${schema}.${opts.table}` : `${n}/${opts.table}`;
  let partition = opts.partition ?? "";
  let run = async () => {
    try {
      await stmt.execute();
    } catch (e) {
      await recordMaterialization(assetPath, partition, "failed", null, null, String(e));
      throw e;
    }
    let snapshot_id: number | null = null;
    let row_count: number | null = null;
    try {
      let s: any = await summary.fetchOne();
      snapshot_id = s?.snapshot_id ?? null;
      row_count = s?.rows ?? null;
    } catch {
      /* summary read is best-effort */
    }
    await recordMaterialization(assetPath, partition, "materialized", snapshot_id, row_count, null);
  };
  return {
    ...stmt,
    execute: (() => run()) as any,
    fetch: (() => run()) as any,
    fetchOne: (() => run()) as any,
    fetchOneScalar: (() => run()) as any,
  };
}

async function recordMaterialization(
  assetPath: string,
  partition: string,
  status: string,
  snapshot_id: number | null,
  row_count: number | null,
  error: string | null,
): Promise<void> {
  try {
    await fetch(`${OpenAPI.BASE}/w/${getWorkspace()}/assets/record_materialization`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        authorization: `Bearer ${OpenAPI.TOKEN as string}`,
      },
      body: JSON.stringify({
        asset_kind: "ducklake",
        asset_path: assetPath,
        partition,
        status,
        snapshot_id,
        row_count,
        job_id: getEnv("WM_JOB_ID") ?? null,
        error,
      }),
    });
  } catch {
    // best-effort; never fail the user's materialization
  }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

// DuckDB executor requires explicit argument types at declaration
// And postgres at argument usage.
// These types exist in both DuckDB and Postgres
// Check that the types exist if you plan to extend this function for other SQL engines.
function inferSqlType(value: any): string {
  if (typeof value === "bigint") return "BIGINT";
  if (typeof value === "number") {
    if (Number.isInteger(value)) return "BIGINT";
    return "DOUBLE PRECISION";
  } else if (value === null || value === undefined) {
    return "TEXT";
  } else if (typeof value === "string") {
    return "TEXT";
  } else if (Array.isArray(value)) {
    // Homogeneous-primitive arrays auto-tag as `TYPE[]` so that values like
    // `${[1,2,3]}` against an `int[]` column work without an explicit
    // `${arr}::int[]` cast. For non-homogeneous or nested arrays we fall
    // back to JSON, which works for jsonb columns.
    return inferSqlArrayType(value);
  } else if (value instanceof Date) {
    // JS `Date` carries an absolute instant in UTC; map to TIMESTAMPTZ so
    // `${someDate}` works against a `timestamptz` column without the user
    // needing an explicit cast. Without this the typeof check above falls
    // through to "object" → JSON, which only works by accident via
    // PG's `json → text → timestamptz` implicit cast chain.
    return "TIMESTAMPTZ";
  } else if (typeof value === "object") {
    return "JSON";
  } else if (typeof value === "boolean") {
    return "BOOLEAN";
  } else {
    return "TEXT";
  }
}

function inferSqlArrayType(value: any[]): string {
  if (value.length === 0) return "JSON";
  // Detect a single shared scalar JS type across all elements. Mixed types
  // or any non-primitive element forces the JSON fallback.
  let scalarType: string | undefined = undefined;
  for (const elem of value) {
    let elemType: string;
    if (typeof elem === "bigint") elemType = "BIGINT";
    else if (typeof elem === "number")
      elemType = Number.isInteger(elem) ? "BIGINT" : "DOUBLE PRECISION";
    else if (typeof elem === "string") elemType = "TEXT";
    else if (typeof elem === "boolean") elemType = "BOOLEAN";
    else return "JSON";
    if (scalarType === undefined) scalarType = elemType;
    else if (scalarType === "BIGINT" && elemType === "DOUBLE PRECISION")
      scalarType = "DOUBLE PRECISION";
    else if (scalarType === "DOUBLE PRECISION" && elemType === "BIGINT") {
      // already widened
    } else if (scalarType !== elemType) {
      return "JSON";
    }
  }
  return `${scalarType}[]`;
}

// The goal is to detect if the user added a type annotation manually
//
// untyped : sql`SELECT ${x} = 0` => ['SELECT ', ' = 0']
// typed   : sql`SELECT ${x}::int = 0` => ['SELECT ', '::int = 0']
// typed   : sql`SELECT CAST ( ${x} AS int ) = 0` => ['SELECT CAST ( ', ' AS int ) = 0']
//
// Caveat: the returned string is only meaningful as a *presence* signal —
// the only consumer (`formatArgUsage`) just checks `explicitType !== undefined`
// to decide whether to emit `$N` (user already wrote a cast) vs `$N::TYPE`
// (SDK injects the inferred cast). The returned string itself can be
// imprecise — e.g. `${x}::DOUBLE PRECISION` returns `"DOUBLE"` (split on
// whitespace), and `CAST(${x} AS int)` returns `"int)"` (no paren stripping).
// Don't rely on the returned string as a parsed PG type; only on whether
// it's defined.
function parseTypeAnnotation(
  prevTemplateString: string | undefined,
  nextTemplateString: string | undefined
): string | undefined {
  if (!nextTemplateString) return;
  nextTemplateString = nextTemplateString.trimStart();
  if (nextTemplateString.startsWith("::")) {
    return nextTemplateString.substring(2).trimStart().split(/\s+/)[0];
  }
  prevTemplateString = prevTemplateString?.trimEnd();
  if (
    prevTemplateString?.endsWith("(") &&
    prevTemplateString
      .substring(0, prevTemplateString.length - 1)
      .trim()
      .toUpperCase()
      .endsWith("CAST") &&
    nextTemplateString.toUpperCase().startsWith("AS ")
  ) {
    return nextTemplateString.substring(2).trimStart().split(/\s+/)[0];
  }
}

function parseName(name: string | undefined): {
  name: string;
  schema?: string;
} {
  if (!name) return { name: "main" };
  let [assetName, schemaName] = name.split(":");
  if (schemaName) {
    return {
      name: assetName || "main",
      schema: schemaName,
    };
  } else {
    return { name };
  }
}
