export type ScriptLanguage =
  | "python3"
  | "deno"
  | "bun"
  | "nativets"
  | "go"
  | "bash"
  | "powershell"
  | "postgresql"
  | "mysql"
  | "bigquery"
  | "duckdb"
  | "oracledb"
  | "snowflake"
  | "mssql"
  | "graphql"
  | "php"
  | "rust"
  | "csharp"
  | "nu"
  | "ansible"
  | "java";
// for related places search: ADD_NEW_LANG

// To make language support raw requirements:
// 1. Add value here
// 2. Modify backend to allow raw deps
export type LanguageWithRawReqsSupport =
  | { language: "bun", rrFilename /** (raw requirements filename) */: "package.json" }
  // TODO: Add `requirements.in` - more intuitive and reflects better what actually happens
  | { language: "python3", rrFilename: "requirements.txt" }
  | { language: "php", rrFilename: "composer.json" }
  | { language: "go", rrFilename: "go.mod" };

export const languagesWithRawReqsSupport: LanguageWithRawReqsSupport[] = [
  { language: "bun", rrFilename: "package.json" },
  { language: "python3", rrFilename: "requirements.txt" },
  { language: "php", rrFilename: "composer.json" },
  { language: "go", rrFilename: "go.mod" },
] as const;

export function inferContentTypeFromFilePath(
  contentPath: string,
  defaultTs: "bun" | "deno" | undefined
): ScriptLanguage {
  if (contentPath.endsWith(".py")) {
    return "python3";
  } else if (contentPath.endsWith("fetch.ts")) {
    return "nativets";
  } else if (contentPath.endsWith("bun.ts")) {
    return "bun";
  } else if (contentPath.endsWith("deno.ts")) {
    return "deno";
  } else if (contentPath.endsWith(".ts")) {
    return defaultTs ?? "bun";
  } else if (contentPath.endsWith(".go")) {
    return "go";
  } else if (contentPath.endsWith(".my.sql")) {
    return "mysql";
  } else if (contentPath.endsWith(".bq.sql")) {
    return "bigquery";
  } else if (contentPath.endsWith(".odb.sql")) {
    return "oracledb";
  } else if (contentPath.endsWith(".duckdb.sql")) {
    return "duckdb";
  } else if (contentPath.endsWith(".sf.sql")) {
    return "snowflake";
  } else if (contentPath.endsWith(".ms.sql")) {
    return "mssql";
  } else if (contentPath.endsWith(".pg.sql")) {
    return "postgresql";
  } else if (contentPath.endsWith(".gql")) {
    return "graphql";
  } else if (contentPath.endsWith(".sh")) {
    return "bash";
  } else if (contentPath.endsWith(".ps1")) {
    return "powershell";
  } else if (contentPath.endsWith(".php")) {
    return "php";
  } else if (contentPath.endsWith(".rs")) {
    return "rust";
  } else if (contentPath.endsWith(".cs")) {
    return "csharp";
  } else if (contentPath.endsWith(".playbook.yml")) {
    return "ansible";
  } else if (contentPath.endsWith(".nu")) {
    return "nu";
  } else if (contentPath.endsWith(".java")) {
    return "java";
    // for related places search: ADD_NEW_LANG
  } else {
    throw new Error(
      "Invalid language: " + contentPath.substring(contentPath.lastIndexOf("."))
    );
  }
}
