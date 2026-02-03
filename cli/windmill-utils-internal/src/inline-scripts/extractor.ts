import { newPathAssigner, PathAssigner } from "../path-utils/path-assigner.ts";
import { FlowModule } from "../gen/types.gen.ts";

/**
 * Represents an inline script extracted from a flow module
 */
interface InlineScript {
  /** File path where the script content should be written */
  path: string;
  /** The actual script content */
  content: string;
}

/**
 * Options for extractInlineScripts function
 */
export interface ExtractInlineScriptsOptions {
  /** When true, skip the .inline_script. suffix in file names */
  skipInlineScriptSuffix?: boolean;
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

  return modules.flatMap((m) => {
    if (m.value.type == "rawscript") {
      const [basePath, ext] = assigner.assignPath(m.summary, m.value.language);
      const path = mapping[m.id] ?? basePath + ext;
      const content = m.value.content;
      const r = [{ path: path, content: content }];
      m.value.content = "!inline " + path.replaceAll(separator, "/");
      const lock = m.value.lock;
      if (lock && lock != "") {
        const lockPath = basePath + "lock";
        m.value.lock = "!inline " + lockPath.replaceAll(separator, "/");
        r.push({ path: lockPath, content: lock });
      }
      return r;
    } else if (m.value.type == "forloopflow") {
      return extractInlineScripts(
        m.value.modules,
        mapping,
        separator,
        defaultTs,
        assigner
      );
    } else if (m.value.type == "branchall") {
      return m.value.branches.flatMap((b) =>
        extractInlineScripts(b.modules, mapping, separator, defaultTs, assigner)
      );
    } else if (m.value.type == "whileloopflow") {
      return extractInlineScripts(
        m.value.modules,
        mapping,
        separator,
        defaultTs,
        assigner
      );
    } else if (m.value.type == "branchone") {
      return [
        ...m.value.branches.flatMap((b) =>
          extractInlineScripts(
            b.modules,
            mapping,
            separator,
            defaultTs,
            assigner
          )
        ),
        ...extractInlineScripts(
          m.value.default,
          mapping,
          separator,
          defaultTs,
          assigner
        ),
      ];
    } else if (m.value.type == "aiagent") {
      return (m.value.tools ?? []).flatMap((tool: any) => {
        const toolValue = tool.value;
        // Only process flowmodule tools with rawscript type
        if (!toolValue || toolValue.tool_type === 'mcp' || toolValue.tool_type === 'websearch') {
          return [];
        }
        if (toolValue.type !== "rawscript") {
          return [];
        }

        const [basePath, ext] = assigner.assignPath(tool.summary ?? tool.id, toolValue.language);
        const path = mapping[tool.id] ?? basePath + ext;
        const content = toolValue.content;
        const r = [{ path: path, content: content }];
        toolValue.content = "!inline " + path.replaceAll(separator, "/");
        const lock = toolValue.lock;
        if (lock && lock != "") {
          const lockPath = basePath + "lock";
          toolValue.lock = "!inline " + lockPath.replaceAll(separator, "/");
          r.push({ path: lockPath, content: lock });
        }
        return r;
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
  mapping: Record<string, string> = {}
): Record<string, string> {
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
      (m.value.tools ?? []).forEach((tool: any) => {
        const toolValue = tool.value;
        if (!toolValue || toolValue.tool_type === 'mcp' || toolValue.tool_type === 'websearch') {
          return;
        }
        if (toolValue.type === "rawscript" && toolValue.content && toolValue.content.startsWith("!inline ")) {
          mapping[tool.id] = toolValue.content.trim().split(" ")[1];
        }
      });
    }
  });

  return mapping;
}
