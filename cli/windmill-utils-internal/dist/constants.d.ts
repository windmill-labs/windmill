/**
 * Cross-platform path separator constants
 * Compatible with both Node.js and Deno environments
 */
/**
 * Path separator constant - equivalent to Deno's SEPARATOR or Node's path.sep
 * On Windows: "\"
 * On Unix-like systems: "/"
 */
export declare const SEP: string;
/**
 * Path delimiter constant for environment variables like PATH
 * On Windows: ";"
 * On Unix-like systems: ":"
 */
export declare const DELIMITER: string;
