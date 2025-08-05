import { newPathAssigner } from "../path-utils/path-assigner";
import { FlowModule } from "../gen/types.gen";

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
 * Extracts inline scripts from flow modules, converting them to separate files
 * and replacing the original content with file references.
 *
 * @param modules - Array of flow modules to process
 * @param mapping - Optional mapping of module IDs to custom file paths
 * @param defaultTs - Default TypeScript runtime to use ("bun" or "deno")
 * @returns Array of inline scripts with their paths and content
 */
export function extractInlineScripts(
  modules: FlowModule[],
  mapping: Record<string, string> = {},
  separator: string = "/",
  defaultTs?: "bun" | "deno"
): InlineScript[] {
  const pathAssigner = newPathAssigner(defaultTs ?? "bun");
  return modules.flatMap((m) => {
    if (m.value.type == "rawscript") {
      let basePath, ext;
      [basePath, ext] = pathAssigner.assignPath(m.summary, m.value.language);
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
        defaultTs
      );
    } else if (m.value.type == "branchall") {
      return m.value.branches.flatMap((b) =>
        extractInlineScripts(b.modules, mapping, separator, defaultTs)
      );
    } else if (m.value.type == "whileloopflow") {
      return extractInlineScripts(
        m.value.modules,
        mapping,
        separator,
        defaultTs
      );
    } else if (m.value.type == "branchone") {
      return [
        ...m.value.branches.flatMap((b) =>
          extractInlineScripts(b.modules, mapping, separator, defaultTs)
        ),
        ...extractInlineScripts(m.value.default, mapping, separator, defaultTs),
      ];
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
    }
  });

  return mapping;
}
