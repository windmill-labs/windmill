/**
 * @param {string} code
 * @returns {string}
 */
export function parse_deno(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_outputs(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_ts_imports(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_bash(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_powershell(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_go(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_python(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_sql(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_mysql(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_bigquery(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_snowflake(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_mssql(code: string): string;
/**
 * @param {string} code
 * @returns {string | undefined}
 */
export function parse_db_resource(code: string): string | undefined;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_graphql(code: string): string;
/**
 * @param {string} code
 * @returns {string}
 */
export function parse_php(code: string): string;
export function instantiate(opts: any): Promise<{
    parse_deno: typeof parse_deno;
    parse_outputs: typeof parse_outputs;
    parse_ts_imports: typeof parse_ts_imports;
    parse_bash: typeof parse_bash;
    parse_powershell: typeof parse_powershell;
    parse_go: typeof parse_go;
    parse_python: typeof parse_python;
    parse_sql: typeof parse_sql;
    parse_mysql: typeof parse_mysql;
    parse_bigquery: typeof parse_bigquery;
    parse_snowflake: typeof parse_snowflake;
    parse_mssql: typeof parse_mssql;
    parse_db_resource: typeof parse_db_resource;
    parse_graphql: typeof parse_graphql;
    parse_php: typeof parse_php;
}>;
export function instantiateWithInstance(opts: any): Promise<{
    instance: any;
    exports: {
        parse_deno: typeof parse_deno;
        parse_outputs: typeof parse_outputs;
        parse_ts_imports: typeof parse_ts_imports;
        parse_bash: typeof parse_bash;
        parse_powershell: typeof parse_powershell;
        parse_go: typeof parse_go;
        parse_python: typeof parse_python;
        parse_sql: typeof parse_sql;
        parse_mysql: typeof parse_mysql;
        parse_bigquery: typeof parse_bigquery;
        parse_snowflake: typeof parse_snowflake;
        parse_mssql: typeof parse_mssql;
        parse_db_resource: typeof parse_db_resource;
        parse_graphql: typeof parse_graphql;
        parse_php: typeof parse_php;
    };
}>;
export function isInstantiated(): boolean;
export function cacheToLocalDir(url: any, decompress: any): Promise<any>;
export function fetchWithRetries(url: any, maxRetries?: number): Promise<Response>;
//# sourceMappingURL=windmill_parser_wasm.generated.d.ts.map