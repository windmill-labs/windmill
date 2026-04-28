/**
 * Snapshot / tag / restore PathScript modules across the dev-page round-trip.
 *
 * **This file is a port of the windmill-vscode extension's
 * `src/utils/pathscript-restore.ts`.** Keep them in sync — divergence here
 * means the same flow round-trips differently in VS Code and Claude Code's
 * local dev preview, which is exactly the kind of bug we worked hard to
 * make impossible.
 *
 * ## Why this exists
 *
 * The dev page can't render PathScript modules (`type: 'script'`, `path:` ref)
 * directly — it has no way to fetch the referenced script's body. Before
 * sending a flow to the page we inline every PathScript's content via
 * `replaceAllPathScriptsWithLocal`, turning each into a rawscript shape the
 * page can render in its editor pane.
 *
 * If we did nothing else, the page would round-trip those rawscripts back
 * verbatim — silently overwriting the user's original `path:` references
 * with frozen snapshots. Reusability gone.
 *
 * The protocol:
 *   1. `snapshotPathScripts` — stash original PathScript values on each
 *      module (and AI-agent tool) before inlining.
 *   2. *(Caller runs `replaceAllPathScriptsWithLocal` here.)*
 *   3. `tagReplacedPathScripts` — move the snapshot inside `value{}` so it
 *      rides along through serialization. The dev page treats it as opaque.
 *   4. *(Page round-trips the flow back over WS / postMessage.)*
 *   5. `restorePathScripts` — swap `value` back to the saved snapshot.
 *      The user's edits to the inlined body are deliberately dropped (you
 *      edit a PathScript by opening its file directly, not through the
 *      flow editor).
 *
 * The `_originalPathScript` tag key is the contract between this file and
 * the dev page's serialization. Don't rename it without coordinating.
 */

const TAG_KEY = "_originalPathScript" as const;

interface ModuleVisitor {
  onModule(module: any): void;
  onTool(tool: any): void;
}

/**
 * Recursively walks all modules in a flow value, visiting leaf modules and
 * AI agent tools. Handles branchone, branchall, forloopflow, whileloopflow,
 * and aiagent nesting.
 */
function walkModules(modules: any[], visitor: ModuleVisitor) {
  for (const module of modules) {
    if (!module.value) continue;
    const val = module.value;
    if (val.type === "forloopflow" || val.type === "whileloopflow") {
      walkModules(val.modules, visitor);
    } else if (val.type === "branchall") {
      for (const branch of val.branches ?? []) {
        walkModules(branch.modules, visitor);
      }
    } else if (val.type === "branchone") {
      for (const branch of val.branches ?? []) {
        walkModules(branch.modules, visitor);
      }
      if (val.default) {
        walkModules(val.default, visitor);
      }
    } else if (val.type === "aiagent") {
      for (const tool of val.tools ?? []) {
        visitor.onTool(tool);
      }
    } else {
      visitor.onModule(module);
    }
  }
}

function walkFlow(flowValue: any, visitor: ModuleVisitor) {
  if (flowValue?.modules) walkModules(flowValue.modules, visitor);
  if (flowValue?.failure_module) walkModules([flowValue.failure_module], visitor);
  if (flowValue?.preprocessor_module) walkModules([flowValue.preprocessor_module], visitor);
}

/**
 * Must be called BEFORE `replaceAllPathScriptsWithLocal` to snapshot the
 * original PathScript values onto each module / AI-agent tool.
 */
export function snapshotPathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module.value.type === "script") {
        module[TAG_KEY] = JSON.parse(JSON.stringify(module.value));
      }
    },
    onTool(tool) {
      const tv = tool.value;
      if (tv && "tool_type" in tv && tv.tool_type === "flowmodule" && tv.type === "script") {
        tool[TAG_KEY] = JSON.parse(JSON.stringify(tv));
      }
    },
  });
}

/**
 * After `replaceAllPathScriptsWithLocal` has mutated the flow, call this to
 * move each snapshot from `module[TAG_KEY]` into `module.value[TAG_KEY]` so
 * it survives serialization to the dev page (the page only forwards what's
 * inside `value`).
 */
export function tagReplacedPathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module[TAG_KEY] && module.value.type === "rawscript") {
        module.value[TAG_KEY] = module[TAG_KEY];
        delete module[TAG_KEY];
      } else if (module[TAG_KEY]) {
        // Snapshotted but not replaced (local file not found) — clean up.
        delete module[TAG_KEY];
      }
    },
    onTool(tool) {
      const tv = tool.value;
      if (tool[TAG_KEY] && tv && "tool_type" in tv && tv.tool_type === "flowmodule" && tv.type === "rawscript") {
        tv[TAG_KEY] = tool[TAG_KEY];
        delete tool[TAG_KEY];
      } else if (tool[TAG_KEY]) {
        delete tool[TAG_KEY];
      }
    },
  });
}

/**
 * Restores PathScript modules in a flow returned from the dev page.
 * Any module/tool with a `_originalPathScript` tag inside its `value` gets
 * restored unconditionally; the tag is removed after restoration.
 */
export function restorePathScripts(flowValue: any) {
  walkFlow(flowValue, {
    onModule(module) {
      if (module.value[TAG_KEY]) {
        module.value = module.value[TAG_KEY];
      }
    },
    onTool(tool) {
      if (tool.value?.[TAG_KEY]) {
        tool.value = tool.value[TAG_KEY];
      }
    },
  });
}
