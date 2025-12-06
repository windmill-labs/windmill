import { RawScript } from "../gen/types.gen.ts";

const INLINE_SCRIPT_PREFIX = "inline_script";

/**
 * Union type of all supported programming languages in Windmill
 */
export type SupportedLanguage = RawScript["language"] | "frontend" | "bunnative" | "oracledb" | "rust" | "csharp" | "nu" | "ansible" | "java" | "duckdb";

/**
 * Mapping of supported languages to their file extensions
 */
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
  bunnative: "ts",
  // for related places search: ADD_NEW_LANG
};

/**
 * Gets the appropriate file extension for a given programming language.
 * Handles special cases for TypeScript variants based on the default runtime.
 *
 * @param language - The programming language to get extension for
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns File extension string (without the dot)
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
 * Reverse mapping from file extensions to languages.
 * Used when deriving language from file extension.
 */
export const EXTENSION_TO_LANGUAGE: Record<string, SupportedLanguage> = {
  "py": "python3",
  "bun.ts": "bun",
  "deno.ts": "deno",
  "go": "go",
  "sh": "bash",
  "ps1": "powershell",
  "pg.sql": "postgresql",
  "my.sql": "mysql",
  "bq.sql": "bigquery",
  "odb.sql": "oracledb",
  "sf.sql": "snowflake",
  "ms.sql": "mssql",
  "gql": "graphql",
  "native.ts": "nativets",
  "frontend.js": "frontend",
  "php": "php",
  "rs": "rust",
  "cs": "csharp",
  "nu": "nu",
  "playbook.yml": "ansible",
  "java": "java",
  "duckdb.sql": "duckdb",
  // Plain .ts defaults to bun (will be overridden by defaultTs setting)
  "ts": "bun",
};

/**
 * Gets the language from a file extension.
 *
 * @param ext - File extension (e.g., "py", "ts", "bun.ts")
 * @param defaultTs - Default TypeScript runtime for plain .ts files
 * @returns The language, or undefined if not recognized
 */
export function getLanguageFromExtension(
  ext: string,
  defaultTs: "bun" | "deno" = "bun"
): SupportedLanguage | undefined {
  // Check for compound extensions first (e.g., "bun.ts", "pg.sql")
  const lang = EXTENSION_TO_LANGUAGE[ext];
  if (lang) {
    // For plain .ts, return the default TypeScript runtime
    if (ext === "ts") {
      return defaultTs;
    }
    return lang;
  }
  return undefined;
}

export interface PathAssigner {
  assignPath(summary: string | undefined, language: SupportedLanguage): [string, string];
}

/**
 * Creates a new path assigner for inline scripts.
 *
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns Path assigner function
 */
export function newPathAssigner(defaultTs: "bun" | "deno"): PathAssigner {
  let counter = 0;
  const seen_names = new Set<string>();
  function assignPath(
    summary: string | undefined,
    language: SupportedLanguage
  ): [string, string] {
    let name;

    name = summary?.toLowerCase()?.replaceAll(" ", "_") ?? "";

    let original_name = name;

    if (name == "") {
      original_name = INLINE_SCRIPT_PREFIX;
      name = `${INLINE_SCRIPT_PREFIX}_0`;
    }

    while (seen_names.has(name)) {
      counter++;
      name = `${original_name}_${counter}`;
    }
    seen_names.add(name);

    const ext = getLanguageExtension(language, defaultTs);

    return [`${name}.inline_script.`, ext];
  }
  return { assignPath };
}

/**
 * Creates a new path assigner for raw app runnables.
 * Unlike newPathAssigner, this does NOT add ".inline_script." prefix since
 * everything in raw_app/backend/ is already known to be for inline scripts.
 *
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns Path assigner function
 */
export function newRawAppPathAssigner(defaultTs: "bun" | "deno"): PathAssigner {
  let counter = 0;
  const seen_names = new Set<string>();
  function assignPath(
    summary: string | undefined,
    language: SupportedLanguage
  ): [string, string] {
    let name;

    name = summary?.toLowerCase()?.replaceAll(" ", "_") ?? "";

    let original_name = name;

    if (name == "") {
      original_name = "runnable";
      name = `runnable_0`;
    }

    while (seen_names.has(name)) {
      counter++;
      name = `${original_name}_${counter}`;
    }
    seen_names.add(name);

    const ext = getLanguageExtension(language, defaultTs);

    return [`${name}.`, ext];
  }
  return { assignPath };
}