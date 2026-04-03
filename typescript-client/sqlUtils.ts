import { getWorkspace, workerHasInternalServer } from "./client";
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

function ducklakeProvider(name: string): SqlProvider {
  return {
    providerName: "ducklake",
    language: "duckdb",
    extraArgs: {},
    formatArgDecl: (argNum, argType) => `-- $arg${argNum} (${argType})`,
    formatArgUsage: (argNum) => `$arg${argNum}`,
    preamble: () => `ATTACH 'ducklake://${name}' AS dl;USE dl;\n`,
  };
}

// ---------------------------------------------------------------------------
// Shared template function builder
// ---------------------------------------------------------------------------

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
          .map((info) => [`arg${info.argNum}`, info.value])
      ),
      ...provider.extraArgs,
    };

    async function fetch<ResultCollectionT extends ResultCollection>({
      resultCollection,
    }: FetchParams<ResultCollectionT> = {}) {
      if (resultCollection)
        content = `-- result_collection=${resultCollection}\n${content}`;
      try {
        let result;
        if (workerHasInternalServer()) {
          result = await JobService.runScriptPreviewInline({
            workspace: getWorkspace(),
            requestBody: { args, content, language },
          });
        } else {
          result = await JobService.runScriptPreviewAndWaitResult({
            workspace: getWorkspace(),
            requestBody: { args, content, language },
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
  let sqlFn = buildSqlTemplateFunction(
    datatableProvider(n, schema)
  ) as DatatableSqlTemplateFunction;
  sqlFn.query = (sqlString: string, ...params: any[]) => {
    let arr = Object.assign([sqlString], { raw: [sqlString] });
    return sqlFn(arr, ...params);
  };
  return sqlFn;
}

/**
 * Create a SQL template function for DuckDB/ducklake queries
 * @param name - DuckDB database name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 */
export function ducklake(name: string = "main"): SqlTemplateFunction {
  return buildSqlTemplateFunction(ducklakeProvider(name));
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

// DuckDB executor requires explicit argument types at declaration
// And postgres at argument usage.
// These types exist in both DuckDB and Postgres
// Check that the types exist if you plan to extend this function for other SQL engines.
function inferSqlType(value: any): string {
  if (typeof value === "number" || typeof value === "bigint") {
    if (Number.isInteger(value)) return "BIGINT";
    return "DOUBLE PRECISION";
  } else if (value === null || value === undefined) {
    return "TEXT";
  } else if (typeof value === "string") {
    return "TEXT";
  } else if (typeof value === "object") {
    return "JSON";
  } else if (typeof value === "boolean") {
    return "BOOLEAN";
  } else {
    return "TEXT";
  }
}

// The goal is to detect if the user added a type annotation manually
//
// untyped : sql`SELECT ${x} = 0` => ['SELECT ', ' = 0']
// typed   : sql`SELECT ${x}::int = 0` => ['SELECT ', '::int = 0']
// typed   : sql`SELECT CAST ( ${x} AS int ) = 0` => ['SELECT CAST ( ', ' AS int ) = 0']
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
