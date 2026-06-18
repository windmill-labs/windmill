import { LANGUAGE_EXTENSIONS, newPathAssigner, PathAssigner } from "../path-utils/path-assigner";
import { FlowModule, RawScript, ScriptLang } from "../gen/types.gen";

/**
 * Represents an inline script extracted from a flow module
 */
export interface InlineScript {
  /** File path where the script content should be written */
  path: string;
  /** The actual script content */
  content: string;
  /** The script language */
  language: ScriptLang;
  is_lock: boolean;
}

function extractRawscriptInline(
  id: string,
  summary: string | undefined,
  rawscript: RawScript,
  mapping: Record<string, string>,
  separator: string,
  assigner: PathAssigner,
  failOnInlineDirective: boolean
): InlineScript[] {
  const [basePath, ext] = assigner.assignPath(summary ?? id, rawscript.language);
  const mappedPath = mapping[id];
  const path = mappedPath ?? basePath + ext;
  const language = rawscript.language;
  const content = rawscript.content;
  // Opt-in defensive guard: when extracting from backend-shaped data (i.e.
  // sync pull), a rawscript whose content is itself an `!inline ...` directive
  // means the backend was poisoned by a prior push that sent the unresolved
  // directive as the script body (GIT-871 / #9140). Refuse to write it back
  // to disk. Off by default because callers that operate on YAML-parsed local
  // flows (flow_metadata, dev) legitimately see `!inline foo.ts` as content.
  if (failOnInlineDirective && typeof content === "string" && content.startsWith("!inline ")) {
    throw new Error(
      `Refusing to extract corrupted inline script for module '${id}': ` +
      `rawscript.content is the literal string \`${content.split("\n")[0]}\` ` +
      `instead of script source. The backend's flow_version.value is corrupt — ` +
      `re-push from a known-good local copy to repair it.`
    );
  }
  const r = [{ path: path, content: content, language, is_lock: false}];
  rawscript.content = "!inline " + path.replaceAll(separator, "/");
  const lock = rawscript.lock;
  if (lock && lock != "") {
    // Derive lock path base from the mapped content path when available,
    // so lock files are named consistently with their content files.
    const lockBasePath = mappedPath
      ? lockBasePathForContent(mappedPath, language)
      : basePath;
    const lockPath = lockBasePath + "lock";
    rawscript.lock = "!inline " + lockPath.replaceAll(separator, "/");
    r.push({ path: lockPath, content: lock, language, is_lock: true});
  }
  return r;
}

/**
 * Derive the lock path base (including trailing dot) from an inline script's
 * content path. The full language extension must be stripped — for compound
 * extensions like "deno.ts", stripping only the last dot segment would yield
 * "x.inline_script.deno.lock" while the assigner-based branch (no mapping,
 * e.g. fresh sync pull) yields "x.inline_script.lock", flip-flopping the lock
 * filename between operations. See legacyLockPathForContent for the old name.
 */
function lockBasePathForContent(contentPath: string, language: ScriptLang): string {
  const langExt = LANGUAGE_EXTENSIONS[language];
  if (langExt && contentPath.endsWith("." + langExt)) {
    return contentPath.substring(0, contentPath.length - langExt.length);
  }
  // Collapsed "ts" (language == defaultTs) or a custom extension: a single
  // dot segment is the whole extension.
  const dotIdx = contentPath.lastIndexOf(".");
  return dotIdx > 0 ? contentPath.substring(0, dotIdx + 1) : contentPath + ".";
}

/**
 * Lock path that CLI versions between #8561 and this fix produced for a given
 * content path (last dot segment stripped, e.g. "x.inline_script.deno.lock").
 * Returns undefined when it matches the canonical name. Callers use this to
 * clean up the stale legacy file after writing the canonical one.
 */
export function legacyLockPathForContent(contentPath: string, language: ScriptLang): string | undefined {
  const dotIdx = contentPath.lastIndexOf(".");
  const legacy = (dotIdx > 0 ? contentPath.substring(0, dotIdx + 1) : contentPath + ".") + "lock";
  const canonical = lockBasePathForContent(contentPath, language) + "lock";
  return legacy === canonical ? undefined : legacy;
}

/**
 * Options for extractInlineScripts function
 */
export interface ExtractInlineScriptsOptions {
  /** When true, skip the .inline_script. suffix in file names */
  skipInlineScriptSuffix?: boolean;
  /**
   * When true, throw if a `rawscript.content` is itself an `!inline ...`
   * directive. Set this only at the sync-pull call site, where the input
   * comes from the backend's `flow_version.value` and `!inline ...` content
   * means the row is corrupt (GIT-871 / #9140). Leave off for callers that
   * pass YAML-parsed local flows — the directive is the legitimate on-disk
   * shape there.
   */
  failOnInlineDirective?: boolean;
}

/**
 * Extracts inline scripts from flow modules, converting them to separate files
 * and replacing the original content with file references.
 *
 * @param modules - Array of flow modules to process
 * @param mapping - Optional mapping of module IDs to custom file paths
 * @param separator - Path separator to use
 * @param defaultTs - Default TypeScript runtime to use ("bun" or "deno")
 * @param pathAssigner - Optional path assigner to reuse (for nested calls)
 * @param options - Optional configuration options
 * @returns Array of inline scripts with their paths and content
 */
export function extractInlineScripts(
  modules: FlowModule[],
  mapping: Record<string, string> = {},
  separator: string = "/",
  defaultTs?: "bun" | "deno",
  pathAssigner?: PathAssigner,
  options?: ExtractInlineScriptsOptions
): InlineScript[] {
  // Create pathAssigner only if not provided (top-level call), but reuse it for nested calls
  const assigner = pathAssigner ?? newPathAssigner(defaultTs ?? "bun", { skipInlineScriptSuffix: options?.skipInlineScriptSuffix });
  const failOnInlineDirective = options?.failOnInlineDirective ?? false;

  return modules.flatMap((m) => {
    if (m.value.type == "rawscript") {
      return extractRawscriptInline(
        m.id,
        m.summary,
        m.value,
        mapping,
        separator,
        assigner,
        failOnInlineDirective
      );
    } else if (m.value.type == "forloopflow") {
      return extractInlineScripts(
        m.value.modules,
        mapping,
        separator,
        defaultTs,
        assigner,
        options
      );
    } else if (m.value.type == "branchall") {
      return m.value.branches.flatMap((b) =>
        extractInlineScripts(b.modules, mapping, separator, defaultTs, assigner, options)
      );
    } else if (m.value.type == "whileloopflow") {
      return extractInlineScripts(
        m.value.modules,
        mapping,
        separator,
        defaultTs,
        assigner,
        options
      );
    } else if (m.value.type == "branchone") {
      return [
        ...m.value.branches.flatMap((b) =>
          extractInlineScripts(
            b.modules,
            mapping,
            separator,
            defaultTs,
            assigner,
            options
          )
        ),
        ...extractInlineScripts(
          m.value.default,
          mapping,
          separator,
          defaultTs,
          assigner,
          options
        ),
      ];
    } else if (m.value.type == "aiagent") {
      return (m.value.tools ?? []).flatMap((tool) => {
        const toolValue = tool.value;
        // Only process flowmodule tools with rawscript type
        if (!toolValue || toolValue.tool_type !== 'flowmodule' || toolValue.type !== 'rawscript') {
          return [];
        }

        return extractRawscriptInline(
          tool.id,
          tool.summary,
          toolValue,
          mapping,
          separator,
          assigner,
          failOnInlineDirective
        );
      });
    } else {
      return [];
    }
  });
}

/**
 * Extracts the current mapping of module IDs to file paths from flow modules
 * by analyzing existing inline script references.
 *
 * @param modules - Array of flow modules to analyze (can be undefined)
 * @param mapping - Existing mapping to extend (defaults to empty object)
 * @returns Record mapping module IDs to their corresponding file paths
 */
export function extractCurrentMapping(
  modules: FlowModule[] | undefined,
  mapping: Record<string, string> = {},
  failureModule?: FlowModule,
  preprocessorModule?: FlowModule,
): Record<string, string> {
  if (failureModule) {
    extractCurrentMapping([failureModule], mapping);
  }
  if (preprocessorModule) {
    extractCurrentMapping([preprocessorModule], mapping);
  }

  if (!modules || !Array.isArray(modules)) {
    return mapping;
  }

  modules.forEach((m) => {
    if (!m?.value?.type) {
      return;
    }

    if (m.value.type === "rawscript") {
      if (m.value.content && m.value.content.startsWith("!inline ")) {
        mapping[m.id] = m.value.content.trim().split(" ")[1];
      }
    } else if (
      m.value.type === "forloopflow" ||
      m.value.type === "whileloopflow"
    ) {
      extractCurrentMapping(m.value.modules, mapping);
    } else if (m.value.type === "branchall") {
      m.value.branches.forEach((b) =>
        extractCurrentMapping(b.modules, mapping)
      );
    } else if (m.value.type === "branchone") {
      m.value.branches.forEach((b) =>
        extractCurrentMapping(b.modules, mapping)
      );
      extractCurrentMapping(m.value.default, mapping);
    } else if (m.value.type === "aiagent") {
      (m.value.tools ?? []).forEach((tool) => {
        const toolValue = tool.value;
        if (!toolValue || toolValue.tool_type !== 'flowmodule' || toolValue.type !== 'rawscript' || !toolValue.content || !toolValue.content.startsWith("!inline ")) {
          return;
        }
        mapping[tool.id] = toolValue.content.trim().split(" ")[1];
      });
    }
  });

  return mapping;
}
