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
  | "snowflake"
  | "mssql"
  | "graphql"
  | "php"
  | "rust"
  | "ansible";

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
  } else if (contentPath.endsWith(".playbook.yml")) {
    return "ansible";
  } else {
    throw new Error(
      "Invalid language: " + contentPath.substring(contentPath.lastIndexOf("."))
    );
  }
}
