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

export declare function datatable(name: string): SqlTemplateFunction;
export declare function ducklake(name: string): SqlTemplateFunction;
