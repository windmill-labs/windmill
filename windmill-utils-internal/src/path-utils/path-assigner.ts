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

export function getLanguageExtension(
  language: SupportedLanguage, 
  defaultTs: "bun" | "deno" = "bun"
): string {
  if (language === defaultTs || language === "bunnative") {
    return "ts";
  }
  return LANGUAGE_EXTENSIONS[language] || "no_ext";
}

export function assignPath(
  id: string,
  language: SupportedLanguage,
  defaultTs: "bun" | "deno" = "bun"
): [string, string] {
  const ext = getLanguageExtension(language, defaultTs);
  return [`${id}.inline_script.`, ext];
}