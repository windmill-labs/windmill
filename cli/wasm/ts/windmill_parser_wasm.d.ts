/* tslint:disable */
/* eslint-disable */
/**
* @param {string} code
* @param {string | undefined} [main_override]
* @param {boolean | undefined} [skip_params]
* @returns {string}
*/
export function parse_deno(code: string, main_override?: string, skip_params?: boolean): string;
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
