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

export function workspaceDependenciesPathToLanguageAndFilename(path: string): { name: string | undefined, language: ScriptLanguage } | undefined {
  const relativePath = path.replace("dependencies/", "");
  for (const { filename, language } of workspaceDependenciesLanguages) {
    if (relativePath.endsWith(filename)) {
      return {
        name: relativePath === filename ? undefined : relativePath.replace("." + filename, ""),
        language
      };
    }
  }
}

// ---------------------------------------------------------------------------
// Annotation parser — mirrors backend's WorkspaceDependenciesAnnotatedRefs::parse
// (windmill-common/src/workspace_dependencies.rs), so the CLI can tell which
// workspace dependency file a script resolves against without asking a worker.
// ---------------------------------------------------------------------------

export type AnnotationMode = "manual" | "extra";

export interface WorkspaceDepsAnnotation {
  mode: AnnotationMode;
  external: string[];
  inline: string | null;
}

const LANG_ANNOTATION_CONFIG: Partial<
  Record<ScriptLanguage, { comment: string; keyword: string; validityRe?: RegExp }>
> = {
  python3: { comment: "#", keyword: "requirements", validityRe: /^#\s?(\S+)\s*$/ },
  bun: { comment: "//", keyword: "package_json" },
  nativets: { comment: "//", keyword: "package_json" },
  go: { comment: "//", keyword: "go_mod" },
  php: { comment: "//", keyword: "composer_json" },
  powershell: { comment: "#", keyword: "modules_json" },
};

export function extractWorkspaceDepsAnnotation(
  scriptContent: string,
  language: ScriptLanguage,
): WorkspaceDepsAnnotation | null {
  const config = LANG_ANNOTATION_CONFIG[language];
  if (!config) return null;

  const { comment, keyword, validityRe } = config;
  const extraMarkerUnderscore = `extra_${keyword}:`;
  const extraMarkerHyphen = `extra-${keyword}:`;
  const manualMarker = `${keyword}:`;

  const stripComment = (l: string): string | null => {
    if (!l.startsWith(comment)) return null;
    return l.substring(comment.length).trimStart();
  };
  const isExtra = (l: string): boolean => {
    const s = stripComment(l);
    return s !== null && (s.startsWith(extraMarkerUnderscore) || s.startsWith(extraMarkerHyphen));
  };
  const isManual = (l: string): boolean => {
    const s = stripComment(l);
    return s !== null && s.startsWith(manualMarker);
  };

  const lines = scriptContent.split("\n");

  // Find first annotation line (mirrors Rust find_position)
  let pos = -1;
  for (let i = 0; i < lines.length; i++) {
    if (isExtra(lines[i]) || isManual(lines[i])) {
      pos = i;
      break;
    }
  }
  if (pos === -1) return null;

  const annotationLine = lines[pos];
  const mode: AnnotationMode = isExtra(annotationLine) ? "extra" : "manual";

  // Parse external references from the annotation line
  const marker = mode === "extra"
    ? (annotationLine.includes(extraMarkerUnderscore) ? extraMarkerUnderscore : extraMarkerHyphen)
    : manualMarker;
  const unparsed = annotationLine.replaceAll(marker, "").replaceAll(comment, "");
  const external = unparsed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  // Parse inline deps from subsequent lines
  const inlineParts: string[] = [];
  for (let i = pos + 1; i < lines.length; i++) {
    const l = lines[i];
    if (validityRe) {
      const match = validityRe.exec(l);
      if (match && match[1]) {
        inlineParts.push(match[1]);
      } else {
        break;
      }
    } else {
      if (!l.startsWith(comment)) {
        break;
      }
      inlineParts.push(l.substring(comment.length));
    }
  }

  const inlineStr = inlineParts.join("\n");
  const inline = inlineStr.trim().length > 0 ? inlineStr : null;

  return { mode, external, inline };
}

/** The comment marker each language's annotations are written behind. */
export const LANG_COMMENT_LIT: Partial<Record<ScriptLanguage, string>> = {
  python3: "#",
  ansible: "#",
  powershell: "#",
  bun: "//",
  nativets: "//",
  deno: "//",
  go: "//",
  php: "//",
  rust: "//!",
};

/**
 * The annotations each language recognises, by the exact names the worker
 * matches (`#[annotations(..)]` structs in windmill-common/src/worker.rs).
 * Several change what it locks — a pinned interpreter, `npm`, `nobundling` —
 * and the rest are cheap to treat the same way, since the only cost is that
 * such a script keeps a lockfile of its own.
 * for related places search: ADD_NEW_LANG
 */
const LANG_ANNOTATIONS: Partial<Record<ScriptLanguage, string[]>> = {
  python3: [
    "no_cache",
    "no_postinstall",
    "py_select_latest",
    "skip_result_postprocessing",
    "py310",
    "py311",
    "py312",
    "py313",
    "sandbox",
  ],
  bun: ["npm", "nodejs", "native", "nobundling", "sandbox"],
  nativets: ["npm", "nodejs", "native", "nobundling", "sandbox"],
  deno: ["npm", "nodejs", "native", "nobundling", "sandbox"],
  go: ["go1_22_compat"],
};

/**
 * Whether a script's leading comment block carries an annotation the worker
 * acts on, which means its lock may not be its dependency file's.
 *
 * Matched the way the worker matches: the key is the line, or what precedes the
 * first `=`, and it has to BE one of the names above. Unknown keys are ignored
 * there and so here — which is what keeps `# TODO:` or `# type: ignore` from
 * quietly dropping an ordinary documented script out of deduplication.
 *
 * `# py: <specifier>` is the exception the macro does not cover: the python
 * import parser reads it directly (`windmill-parser-py-imports`, alongside the
 * `py310`..`py313` flags) to pick the interpreter, which changes what resolves.
 */
export function hasLockAffectingAnnotation(
  scriptContent: string,
  language: ScriptLanguage,
): boolean {
  const comment = LANG_COMMENT_LIT[language];
  const names = LANG_ANNOTATIONS[language];
  if (!comment || !names) return false;
  for (const line of scriptContent.split("\n")) {
    const trimmed = line.trim();
    if (trimmed === "") continue;
    if (!trimmed.startsWith(comment)) break; // past the header block
    // Matched on the raw line: the parser tests `# py:`/`#py:` before trimming.
    if (language === "python3" && /^#\s?py:/.test(line)) return true;
    const body = trimmed.slice(comment.length).trim();
    const key = body.split("=")[0].trim();
    if (names.includes(key)) return true;
  }
  return false;
}

/** Where the lockfiles shared by several scripts live when `dedupeLockfiles`
 *  is on — see `utils/lock_dedup.ts`. A top-level directory of its own: what a
 *  group shares is a resolved lock, which needs no workspace dependency file
 *  behind it, and inline-script locks would belong here too. */
export const SHARED_LOCK_DIR = "locks";

/** The lockfile shared by the scripts that resolve against a workspace
 *  dependency file: its own name, plus `.lock`. Appending rather than replacing
 *  the extension keeps the correspondence exact and reversible —
 *  `dependencies/team_a.requirements.in` <-> `locks/team_a.requirements.in.lock`. */
export function sharedLockPathFor(depFilePath: string): string {
  const name = depFilePath.replaceAll("\\", "/").split("/").pop()!;
  return `${SHARED_LOCK_DIR}/${name}.lock`;
}

/** The workspace dependency file a shared lockfile belongs to, if it is one. */
export function depFileOfSharedLock(p: string): string | undefined {
  const normalized = p.replaceAll("\\", "/");
  if (!normalized.startsWith(SHARED_LOCK_DIR + "/")) return undefined;
  const name = normalized.slice(SHARED_LOCK_DIR.length + 1);
  if (name.includes("/") || !name.endsWith(".lock")) return undefined;
  const depFile = "dependencies/" + name.slice(0, -".lock".length);
  const info = workspaceDependenciesPathToLanguageAndFilename(depFile);
  // `locks/vendor.lock` names no dependency file, so it is not Windmill's: a
  // repo that already keeps lockfiles here keeps them.
  return info && languageNeedsLock(info.language) ? depFile : undefined;
}

export function isSharedLockPath(p: string): boolean {
  return depFileOfSharedLock(p) !== undefined;
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
