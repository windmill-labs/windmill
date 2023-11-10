/* tslint:disable */
/* eslint-disable */
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
* @returns {string}
*/
export function parse_graphql(code: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly parse_deno: (a: number, b: number, c: number) => void;
  readonly parse_outputs: (a: number, b: number, c: number) => void;
  readonly parse_bash: (a: number, b: number, c: number) => void;
  readonly parse_powershell: (a: number, b: number, c: number) => void;
  readonly parse_go: (a: number, b: number, c: number) => void;
  readonly parse_python: (a: number, b: number, c: number) => void;
  readonly parse_sql: (a: number, b: number, c: number) => void;
  readonly parse_mysql: (a: number, b: number, c: number) => void;
  readonly parse_bigquery: (a: number, b: number, c: number) => void;
  readonly parse_snowflake: (a: number, b: number, c: number) => void;
  readonly parse_mssql: (a: number, b: number, c: number) => void;
  readonly parse_graphql: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
