/* tslint:disable */
/* eslint-disable */
export function parse_deno(code: string, main_override?: string | null): string;
export function parse_outputs(code: string): string;
/**
 * Parse TypeScript imports and return raw import strings.
 * See [`parse_ts_relative_imports`] for resolved absolute paths.
 */
export function parse_ts_imports(code: string): string;
/**
 * Parse TypeScript imports and return relative imports resolved to absolute Windmill paths.
 * Throws JS error on parse failure.
 * See [`parse_ts_imports`] for raw import strings.
 */
export function parse_ts_relative_imports(code: string, path: string): string[];
export function parse_assets_ts(code: string): string;
