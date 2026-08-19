import { isDbtDescriptorPath } from "./resource_folders.ts";
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
  | "rlang"
  | "dbt"
  | "java";
// for related places search: ADD_NEW_LANG

// To make language support raw requirements:
// 1. Add value here
// 2. Modify backend to allow raw deps
export type WorkspaceDependenciesLanguage =
  | { language: "bun", filename /** (raw requirements filename) */: "package.json" }
  | { language: "python3", filename: "requirements.in" }
  | { language: "php", filename: "composer.json" }
  | { language: "go", filename: "go.mod" }
  | { language: "powershell", filename: "modules.json" };

export const workspaceDependenciesLanguages: WorkspaceDependenciesLanguage[] = [
  { language: "bun", filename: "package.json" },
  { language: "python3", filename: "requirements.in" },
  { language: "php", filename: "composer.json" },
  { language: "go", filename: "go.mod" },
  { language: "powershell", filename: "modules.json" },
] as const;

/** Where the lockfiles shared by several scripts live when `dedupeLockfiles`
 *  is on — see `utils/lock_dedup.ts`. A top-level directory of its own: what a
 *  group shares is a resolved lock, which needs no workspace dependency file
 *  behind it, and inline-script locks would belong here too. */
export const SHARED_LOCK_DIR = "locks";

export function sharedLockPath(language: string, n = 1): string {
  return n === 1
    ? `${SHARED_LOCK_DIR}/${language}.lock`
    : `${SHARED_LOCK_DIR}/${language}-${n}.lock`;
}

/**
 * Whether a path is a shared lockfile Windmill would have written.
 *
 * Narrow on purpose: `locks/` is an ordinary word, so a repo may already have
 * one holding something else. Only the names `sharedLockPath` can produce are
 * claimed — anything else under `locks/` stays invisible to sync, and cannot be
 * swept as an unreferenced shared lock.
 */
export function isSharedLockPath(p: string): boolean {
  const normalized = p.replaceAll("\\", "/");
  if (!normalized.startsWith(SHARED_LOCK_DIR + "/")) return false;
  const name = normalized.slice(SHARED_LOCK_DIR.length + 1);
  if (name.includes("/")) return false;
  const match = /^(.+?)(?:-(\d+))?\.lock$/.exec(name);
  if (!match) return false;
  const n = match[2] === undefined ? 1 : Number(match[2]);
  return (
    languageNeedsLock(match[1]) &&
    n >= 1 &&
    sharedLockPath(match[1], n) === normalized
  );
}

/**
 * Returns true if a script in the given language requires a lock file.
 * Matches the condition in updateScriptLock (metadata.ts).
 */
export function languageNeedsLock(language: ScriptLanguage | string): boolean {
  return (
    (workspaceDependenciesLanguages.some((l) => l.language === language) &&
      language !== "powershell") ||
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
  } else if (isDbtDescriptorPath(contentPath)) {
    return "dbt";
  } else if (contentPath.endsWith(".r")) {
    return "rlang";
	// for related places search: ADD_NEW_LANG
  } else {
    const ext = contentPath.substring(contentPath.lastIndexOf("."));
    let hint = "";
    if (ext === ".sql") {
      hint =
        "\nBare .sql is ambiguous — use a dialect extension: .pg.sql (postgresql), .my.sql (mysql), .bq.sql (bigquery), .sf.sql (snowflake), .ms.sql (mssql), .odb.sql (oracledb), .duckdb.sql (duckdb)";
    }
    throw new Error(
      `Cannot infer script language from extension '${ext}' (file ${contentPath}).` +
        hint +
        "\nSupported extensions: .ts (bun/deno), .py, .go, .sh, .ps1, .php, .rs, .cs, .nu, .java, .rb, .r, .gql, .playbook.yml, .pg.sql, .my.sql, .bq.sql, .sf.sql, .ms.sql, .odb.sql, .duckdb.sql, and a dbt project folder `<name>__dbt/`"
    );
  }
}
