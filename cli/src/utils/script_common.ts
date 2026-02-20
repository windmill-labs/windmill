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
  | "ruby"
  | "java";
// for related places search: ADD_NEW_LANG

// To make language support raw requirements:
// 1. Add value here
// 2. Modify backend to allow raw deps
export type WorkspaceDependenciesLanguage =
  | { language: "bun", filename /** (raw requirements filename) */: "package.json" }
  | { language: "python3", filename: "requirements.in" }
  | { language: "php", filename: "composer.json" }
  | { language: "go", filename: "go.mod" };

export const workspaceDependenciesLanguages: WorkspaceDependenciesLanguage[] = [
  { language: "bun", filename: "package.json" },
  { language: "python3", filename: "requirements.in" },
  { language: "php", filename: "composer.json" },
  { language: "go", filename: "go.mod" },
] as const;

/**
 * Returns true if a script in the given language requires a lock file.
 * Matches the condition in updateScriptLock (metadata.ts).
 */
export function languageNeedsLock(language: ScriptLanguage | string): boolean {
  return (
    workspaceDependenciesLanguages.some((l) => l.language === language) ||
    language === "deno" ||
    language === "rust" ||
    language === "ansible"
  );
}

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
  } else if (contentPath.endsWith(".rb")) {
    return "ruby";
	// for related places search: ADD_NEW_LANG
  } else {
    throw new Error(
      "Invalid language: " + contentPath.substring(contentPath.lastIndexOf("."))
    );
  }
}
