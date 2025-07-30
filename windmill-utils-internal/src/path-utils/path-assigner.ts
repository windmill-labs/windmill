import { RawScript } from "windmill-client";

export type SupportedLanguage = RawScript["language"] | "frontend" | "bunnative" | "oracledb" | "rust" | "csharp" | "nu" | "ansible" | "java" | "duckdb";

export const LANGUAGE_EXTENSIONS: Record<SupportedLanguage, string> = {
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
 * Gets the file extension for a given programming language.
 * 
 * @param language - The programming language
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns The file extension for the language
 */
export function getLanguageExtension(
  language: SupportedLanguage, 
  defaultTs: "bun" | "deno" = "bun"
): string {
  if (language === defaultTs || language === "bunnative") {
    return "ts";
  }
  return LANGUAGE_EXTENSIONS[language] || "no_ext";
}

/**
 * Assigns a file path and extension for an inline script based on its ID and language.
 * 
 * @param id - The unique identifier for the script
 * @param language - The programming language of the script
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns A tuple containing the base path and file extension
 */
export function assignPath(
  id: string,
  language: SupportedLanguage,
  defaultTs: "bun" | "deno" = "bun"
): [string, string] {
  const ext = getLanguageExtension(language, defaultTs);
  return [`${id}.inline_script.`, ext];
}