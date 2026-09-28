/**
 * Flow checks the OpenFlow JSON schema cannot express. Errors are what fails
 * a run of the flow or stops the flow editor from drawing it; warnings are
 * what the editor tolerates but renders wrong.
 */

// `TOOL_NAME_REGEX` and `flow_module_tool_name` in
// backend/windmill-worker/src/ai_executor.rs; `getToolNameError` in
// frontend/src/lib/components/flows/agentToolUtils.ts applies the same rules.
const TOOL_NAME_REGEX = /^[a-zA-Z0-9_]+$/;
const WEBSEARCH_ENABLED_NAME = "__wm_web_search";

// `forbiddenIds` in frontend/src/lib/components/flows/idUtils.ts.
const RESERVED_IDS = new Set([
  "do",
  "bg",
  "ctx",
  "state",
  "if",
  "else",
  "for",
  "delete",
  "while",
  "new",
  "in",
  "failure",
  "preprocessor",
  "__wm_agent_root",
  "as",
  "Input",
  "Result",
  "Trigger",
]);

// `NoteColor` in frontend/src/lib/components/graph/noteColors.ts.
const PALETTE = [
  "yellow",
  "blue",
  "green",
  "purple",
  "pink",
  "orange",
  "red",
  "cyan",
  "lime",
  "gray",
];

// Nodes the flow graph synthesizes (`VIRTUAL_NODE_IDS` in
// frontend/src/lib/components/graph/groupDetectionUtils.ts).
const VIRTUAL_NODE_IDS = new Set(["Input", "Result", "Trigger"]);

export interface FlowSemanticReport {
  errors: string[];
  warnings: string[];
}

interface ModuleList {
  pointer: string;
  modules: any[];
}

// These checks also run on a flow that failed schema validation, so any
// collection may have the wrong type; treating it as empty leaves reporting it
// to the schema errors instead of aborting the whole lint run.
function asArray(value: unknown): any[] {
  return Array.isArray(value) ? value : [];
}

/**
 * Every module list a group can span, and every aiagent module, anywhere in the
 * tree. The lists match `getContainerInnerArrays` in
 * frontend/src/lib/components/graph/groupEditor.svelte.ts: agent tools are not
 * steps of the graph, so no group can reach them.
 */
function walkModules(
  modules: unknown,
  pointer: string,
  lists: ModuleList[],
  agents: { pointer: string; module: any }[],
) {
  if (!Array.isArray(modules)) return;
  lists.push({ pointer, modules });
  modules.forEach((m, i) => {
    const value = m?.value;
    const at = `${pointer}/${i}/value`;
    switch (value?.type) {
      case "forloopflow":
      case "whileloopflow":
        walkModules(value.modules, `${at}/modules`, lists, agents);
        break;
      case "branchone":
        walkModules(value.default, `${at}/default`, lists, agents);
        asArray(value.branches).forEach((b: any, bi: number) =>
          walkModules(b?.modules, `${at}/branches/${bi}/modules`, lists, agents)
        );
        break;
      case "branchall":
        asArray(value.branches).forEach((b: any, bi: number) =>
          walkModules(b?.modules, `${at}/branches/${bi}/modules`, lists, agents)
        );
        break;
      case "aiagent":
        collectAgents(m, `${pointer}/${i}`, agents);
        break;
    }
  });
}

/** An aiagent module plus any agent that is itself one of its tools. */
function collectAgents(
  module: any,
  pointer: string,
  agents: { pointer: string; module: any }[],
) {
  // A step linked to an `ai_agent` resource runs the resource's tools and ignores
  // its own (`ai_executor.rs`), so those are left unchecked.
  if (typeof module?.value?.agent === "string" && module.value.agent !== "") {
    return;
  }
  agents.push({ pointer, module });
  asArray(module?.value?.tools).forEach((tool: any, i: number) => {
    if (tool?.value?.type === "aiagent") {
      collectAgents(tool, `${pointer}/value/tools/${i}`, agents);
    }
  });
}

function checkAgentTools(
  agent: { pointer: string; module: any },
  report: FlowSemanticReport,
) {
  const tools = agent.module?.value?.tools;
  if (!Array.isArray(tools)) return;
  const seen = new Map<string, number>();
  tools.forEach((tool: any, i: number) => {
    // An mcp entry exposes its server's own tool names and a websearch entry is
    // named by `WEBSEARCH_ENABLED_NAME`; neither summary reaches the model.
    const kind = tool?.value?.tool_type;
    if (kind === "mcp" || kind === "websearch") return;
    const at = `${agent.pointer}/value/tools/${i}`;
    const name = tool?.summary;
    if (typeof name !== "string" || name.length === 0) {
      report.errors.push(
        `${at} has no summary: a tool's summary is the name the agent calls it by`,
      );
      return;
    }
    if (!TOOL_NAME_REGEX.test(name)) {
      report.errors.push(
        `${at} tool name '${name}' may only contain letters, numbers and underscores (a run that offers this tool fails with 'Invalid tool name')`,
      );
      return;
    }
    if (name === WEBSEARCH_ENABLED_NAME) {
      report.errors.push(
        `${at} tool name '${name}' is reserved for enabling web search`,
      );
      return;
    }
    if (seen.has(name)) {
      report.errors.push(
        `${at} tool name '${name}' is already used by tool ${seen.get(name)} of this agent`,
      );
      return;
    }
    seen.set(name, i);
    if (RESERVED_IDS.has(name)) {
      report.warnings.push(
        `${at} tool name '${name}' is a reserved id; the flow editor refuses it`,
      );
    }
  });
}

function checkColor(
  at: string,
  color: unknown,
  report: FlowSemanticReport,
) {
  if (color === undefined || color === null) return;
  if (typeof color !== "string" || !PALETTE.includes(color)) {
    report.warnings.push(
      `${at} color '${color}' renders unstyled; use one of: ${PALETTE.join(", ")}`,
    );
  }
}

/**
 * Mirrors `buildStructureTree` in frontend/src/lib/components/graph/flowStructure.ts,
 * whose throws make the editor draw no step at all.
 */
function checkGroups(
  groups: unknown,
  lists: ModuleList[],
  report: FlowSemanticReport,
) {
  if (!Array.isArray(groups)) return;
  const location = new Map<string, { list: number; index: number }>();
  lists.forEach((l, list) =>
    l.modules.forEach((m, index) => {
      if (typeof m?.id === "string") location.set(m.id, { list, index });
    })
  );

  const placed: { at: string; list: number; start: number; end: number }[] =
    [];
  const keys = new Set<string>();
  groups.forEach((g: any, i: number) => {
    const at = `/value/groups/${i}`;
    checkColor(at, g?.color, report);
    const startId = g?.start_id;
    const endId = g?.end_id;
    if (typeof startId !== "string" || typeof endId !== "string") return;

    const key = `${startId}:${endId}`;
    if (keys.has(key)) {
      report.errors.push(
        `${at} spans the same steps as another group (${startId} to ${endId})`,
      );
      return;
    }
    keys.add(key);

    for (const id of [startId, endId]) {
      if (VIRTUAL_NODE_IDS.has(id)) {
        report.errors.push(`${at} cannot include the '${id}' node`);
        return;
      }
      if (!location.has(id)) {
        report.errors.push(
          id === "preprocessor" || id === "failure"
            ? `${at} cannot include the '${id}' module`
            : `${at} references '${id}', which is not a step of this flow`,
        );
        return;
      }
    }
    const start = location.get(startId)!;
    const end = location.get(endId)!;
    if (start.list !== end.list) {
      report.errors.push(
        `${at} starts at '${startId}' and ends at '${endId}', which are not in the same branch`,
      );
      return;
    }
    if (start.index > end.index) {
      report.errors.push(
        `${at} starts at '${startId}', which comes after its end '${endId}'`,
      );
      return;
    }
    placed.push({ at, list: start.list, start: start.index, end: end.index });
  });

  for (let a = 0; a < placed.length; a++) {
    for (let b = a + 1; b < placed.length; b++) {
      const x = placed[a];
      const y = placed[b];
      if (x.list !== y.list) continue;
      const disjoint = x.end < y.start || y.end < x.start;
      const nested = (x.start <= y.start && y.end <= x.end) ||
        (y.start <= x.start && x.end <= y.end);
      if (!disjoint && !nested) {
        report.errors.push(
          `${y.at} partly overlaps ${x.at}: groups may nest but not overlap`,
        );
      }
    }
  }
}

function checkNotes(notes: unknown, report: FlowSemanticReport) {
  if (!Array.isArray(notes)) return;
  const ids = new Set<string>();
  notes.forEach((n: any, i: number) => {
    const at = `/value/notes/${i}`;
    checkColor(at, n?.color, report);
    if (typeof n?.id === "string") {
      if (ids.has(n.id)) {
        report.warnings.push(`${at} reuses the note id '${n.id}'`);
      }
      ids.add(n.id);
    }
    if (n?.type === "group") {
      report.warnings.push(
        `${at} is a 'group' note, which is deprecated: use value.groups instead`,
      );
    } else if (!n?.position || !n?.size) {
      report.warnings.push(
        `${at} has no position or size, so the editor places it at the origin and cannot resize it`,
      );
    }
  });
}

export function checkFlowSemantics(flow: unknown): FlowSemanticReport {
  const report: FlowSemanticReport = { errors: [], warnings: [] };
  const value = (flow as any)?.value;
  if (!value || typeof value !== "object") return report;

  const lists: ModuleList[] = [];
  const agents: { pointer: string; module: any }[] = [];
  walkModules(value.modules, "/value/modules", lists, agents);

  for (const agent of agents) {
    checkAgentTools(agent, report);
  }
  checkGroups(value.groups, lists, report);
  checkNotes(value.notes, report);
  return report;
}
