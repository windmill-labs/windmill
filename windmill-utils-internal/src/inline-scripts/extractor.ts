import { assignPath, SupportedLanguage } from "../path-utils";
import { SEP } from "../constants";
import { FlowModule } from "windmill-client";

interface InlineScript {
  path: string;
  content: string;
}

export function extractInlineScripts(
  modules: FlowModule[],
  mapping: Record<string, string> = {},
  defaultTs?: "bun" | "deno"
): InlineScript[] {
  return modules.flatMap((m) => {
    if (m.value.type == "rawscript") {
      const [basePath, ext] = assignPath(
        m.id,
        m.value.language,
        defaultTs
      );
      const path = mapping[m.id] ?? basePath + ext;
      const content = m.value.content;
      const r = [{ path: path, content: content }];
      m.value.content = "!inline " + path.replace(SEP, "/");
      const lock = m.value.lock;
      if (lock && lock != "") {
        const lockPath = basePath + "lock";
        m.value.lock = "!inline " + lockPath.replace(SEP, "/");
        r.push({ path: lockPath, content: lock });
      }
      return r;
    } else if (m.value.type == "forloopflow") {
      return extractInlineScripts(m.value.modules, mapping, defaultTs);
    } else if (m.value.type == "branchall") {
      return m.value.branches.flatMap((b) =>
        extractInlineScripts(b.modules, mapping, defaultTs)
      );
    } else if (m.value.type == "whileloopflow") {
      return extractInlineScripts(m.value.modules, mapping, defaultTs);
    } else if (m.value.type == "branchone") {
      return [
        ...m.value.branches.flatMap((b) =>
          extractInlineScripts(b.modules, mapping, defaultTs)
        ),
        ...extractInlineScripts(m.value.default, mapping, defaultTs),
      ];
    } else {
      return [];
    }
  });
}

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
      m.value.branches.forEach((b) => extractCurrentMapping(b.modules, mapping));
    } else if (m.value.type === "branchone") {
      m.value.branches.forEach((b) => extractCurrentMapping(b.modules, mapping));
      extractCurrentMapping(m.value.default, mapping);
    }
  });
  
  return mapping;
}