"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.LANGUAGE_EXTENSIONS = void 0;
exports.getLanguageExtension = getLanguageExtension;
exports.assignPath = assignPath;
/**
 * Mapping of supported languages to their file extensions
 */
exports.LANGUAGE_EXTENSIONS = {
    python3: "py",
    bun: "bun.ts",
    deno: "deno.ts",
    go: "go",
    bash: "sh",
    powershell: "ps1",
    postgresql: "pg.sql",
    mysql: "my.sql",
    bigquery: "bq.sql",
    oracledb: "odb.sql",
    snowflake: "sf.sql",
    mssql: "ms.sql",
    graphql: "gql",
    nativets: "native.ts",
    frontend: "frontend.js",
    php: "php",
    rust: "rs",
    csharp: "cs",
    nu: "nu",
    ansible: "playbook.yml",
    java: "java",
    duckdb: "duckdb.sql",
    bunnative: "ts"
};
/**
 * Gets the appropriate file extension for a given programming language.
 * Handles special cases for TypeScript variants based on the default runtime.
 *
 * @param language - The programming language to get extension for
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns File extension string (without the dot)
 */
function getLanguageExtension(language, defaultTs = "bun") {
    if (language === defaultTs || language === "bunnative") {
        return "ts";
    }
    return exports.LANGUAGE_EXTENSIONS[language] || "no_ext";
}
/**
 * Assigns a file path for an inline script based on its ID and language.
 * Returns both the base path and extension as separate components.
 *
 * @param id - Unique identifier for the script
 * @param language - Programming language of the script
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns Tuple containing [basePath, extension]
 */
function assignPath(id, language, defaultTs = "bun") {
    const ext = getLanguageExtension(language, defaultTs);
    return [`${id}.inline_script.`, ext];
}
