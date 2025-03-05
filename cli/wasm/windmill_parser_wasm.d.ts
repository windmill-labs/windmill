// @generated file from wasmbuild -- do not edit
// deno-lint-ignore-file
// deno-fmt-ignore-file

export function parse_deno(
  code: string,
  main_override?: string | null,
  skip_params?: boolean | null,
): string;
export function parse_outputs(code: string): string;
export function parse_ts_imports(code: string): string;
export function parse_bash(code: string): string;
export function parse_powershell(code: string): string;
export function parse_go(code: string): string;
export function parse_python(
  code: string,
  main_override?: string | null,
): string;
export function parse_sql(code: string): string;
export function parse_mysql(code: string): string;
export function parse_oracledb(code: string): string;
export function parse_bigquery(code: string): string;
export function parse_snowflake(code: string): string;
export function parse_mssql(code: string): string;
export function parse_db_resource(code: string): string | undefined;
export function parse_graphql(code: string): string;
export function parse_php(code: string): string;
export function parse_rust(code: string): string;
export function parse_ansible(code: string): string;
export function parse_csharp(code: string): string;
export function parse_nu(code: string): string;
