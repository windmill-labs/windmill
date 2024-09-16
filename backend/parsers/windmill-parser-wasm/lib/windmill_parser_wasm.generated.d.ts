// deno-lint-ignore-file
// deno-fmt-ignore-file

export interface InstantiateResult {
  instance: WebAssembly.Instance;
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
    parse_rust: typeof parse_rust
  };
}

/** Gets if the Wasm module has been instantiated. */
export function isInstantiated(): boolean;

/** Options for instantiating a Wasm instance. */
export interface InstantiateOptions {
  /** Optional url to the Wasm file to instantiate. */
  url?: URL;
  /** Callback to decompress the raw Wasm file bytes before instantiating. */
  decompress?: (bytes: Uint8Array) => Uint8Array;
}

/** Instantiates an instance of the Wasm module returning its functions.
* @remarks It is safe to call this multiple times and once successfully
* loaded it will always return a reference to the same object. */
export function instantiate(opts?: InstantiateOptions): Promise<InstantiateResult["exports"]>;

/** Instantiates an instance of the Wasm module along with its exports.
 * @remarks It is safe to call this multiple times and once successfully
 * loaded it will always return a reference to the same object. */
export function instantiateWithInstance(opts?: InstantiateOptions): Promise<InstantiateResult>;

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
/**
* @param {string} code
* @returns {string}
*/
export function parse_rust(code: string): string;
