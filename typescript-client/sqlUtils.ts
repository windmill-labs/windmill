import { getWorkspace, JobService } from "./client";

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

type SqlResult<ResultCollectionT extends ResultCollection> =
  ResultCollectionT extends "last_statement_first_row"
    ? object
    : ResultCollectionT extends "all_statements_first_row"
    ? object[]
    : ResultCollectionT extends "last_statement_all_rows"
    ? object[]
    : ResultCollectionT extends "all_statements_all_rows"
    ? object[][]
    : ResultCollectionT extends "last_statement_all_rows_scalar"
    ? any[]
    : ResultCollectionT extends "all_statements_all_rows_scalar"
    ? any[][]
    : ResultCollectionT extends "last_statement_first_row_scalar"
    ? any
    : ResultCollectionT extends "all_statements_first_row_scalar"
    ? any[]
    : unknown;
export type SqlStatement = {
  content: string;

  args: Record<string, any>;

  fetch<ResultCollectionT extends ResultCollection = "last_statement_all_rows">(
    params?: FetchParams<ResultCollectionT | ResultCollection> // The union is for auto-completion
  ): Promise<SqlResult<ResultCollectionT>>;

  fetchOne(
    params?: Omit<FetchParams<"last_statement_first_row">, "resultCollection">
  ): Promise<SqlResult<"last_statement_first_row">>;
};

export interface SqlTemplateFunction {
  (strings: TemplateStringsArray, ...values: any[]): SqlStatement;
}

/**
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
export function datatable(name: string = "main"): SqlTemplateFunction {
  return sqlProviderImpl(name, "datatable");
}

/**
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
  return sqlProviderImpl(name, "ducklake");
}

function sqlProviderImpl(
  name: string,
  provider: "datatable" | "ducklake"
): SqlTemplateFunction {
  let sql: SqlTemplateFunction = (
    strings: TemplateStringsArray,
    ...values: any[]
  ) => {
    let formatArg = {
      datatable: (i: number) => `-- $${i + 1} arg${i + 1}`,
      ducklake: (i: number) => `-- $arg${i + 1}`,
    }[provider];
    let content = values.map((_, i) => formatArg(i)).join("\n") + "\n";
    if (provider === "ducklake")
      content += `ATTACH 'ducklake://${name}' AS dl;USE dl;\n`;

    let contentBody = "";
    for (let i = 0; i < strings.length; i++) {
      contentBody += strings[i];
      if (i !== strings.length - 1) contentBody += `$${i + 1}`;
    }
    content += contentBody;

    const args = {
      ...Object.fromEntries(values.map((v, i) => [`arg${i + 1}`, v])),
      ...(provider === "datatable" ? { database: `datatable://${name}` } : {}),
    };
    const language = {
      datatable: "postgresql" as const,
      ducklake: "duckdb" as const,
    }[provider];

    async function fetch<ResultCollectionT extends ResultCollection>({
      resultCollection,
    }: FetchParams<ResultCollectionT> = {}) {
      if (resultCollection)
        content = `-- result_collection=${resultCollection}\n${content}`;
      try {
        let result = await JobService.runScriptPreviewInline({
          workspace: getWorkspace(),
          requestBody: { args, content, language },
        });
        return result as SqlResult<ResultCollectionT>;
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
          err = Error(`${provider} ${body}`);
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
    } satisfies SqlStatement;
  };
  return sql;
}
