/* tslint:disable */
/* eslint-disable */
export function parse_python(code: string, main_override?: string | null): string;
export function parse_assets_py(code: string): string;
/**
 * Parse Python imports and return relative imports resolved to absolute Windmill paths.
 * Throws JS error on parse failure.
 */
export function parse_py_relative_imports(code: string, path: string): string[];
