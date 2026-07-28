import { GlobalOptions, isSuperset } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { dirname, sep as SEP } from "node:path";
import { stringify as yamlStringify } from "yaml";
import { yamlParseFile } from "../../utils/yaml.ts";
import { readTextFile, validateRequiredArgs } from "../../utils/utils.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { mkdirSync, writeFileSync } from "node:fs";
import { buildFolderPath, getMetadataFileName, loadNonDottedPathsSetting } from "../../utils/resource_folders.ts";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { resolve, track_job, pollForJobResult } from "../script/script.ts";
import { defaultFlowDefinition } from "../../../bootstrap/flow_bootstrap.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { FSFSElement, elementsToMap, ignoreF } from "../sync/sync.ts";
import { Flow } from "../../../gen/types.gen.ts";
import { applyExtraPermsDiff } from "../../core/extra_perms.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import {
  collectPathScriptPaths,
  replaceInlineScripts,
  replaceAllPathScriptsWithLocal,
} from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { generateFlowLockInternal } from "./flow_metadata.ts";
import { exts } from "../script/script.ts";
import type { SyncCodebase } from "../../utils/codebase.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import {
  createPreviewLocalScriptReader,
  resolvePreviewLocalScriptState,
} from "../../utils/local_path_scripts.ts";

export interface FlowFile {
  summary: string;
  description?: string;
  value: any;
  schema?: any;
  on_behalf_of_email?: string;
  has_on_behalf_of?: boolean;
  // Mirrors granular ACLs on the flow path. Omitted from flow.yaml when no
  // perms are set. The CLI applies diffs through /acls/add and /acls/remove
  // (see applyExtraPermsDiff) — never through update_flow — so a perm-only
  // change never bumps the flow version.
  extra_perms?: Record<string, boolean>;
}

function normalizeOptionalString(value: string | null | undefined): string | undefined {
  return typeof value === "string" && value.trim() === "" ? undefined : value ?? undefined;
}

function normalizeComparableContent(value: string | undefined): string | undefined {
  return value?.replaceAll("\r\n", "\n").replace(/\n$/, "");
}

async function findDivergedLocalPathScripts(
  workspaceId: string,
  scriptPaths: string[],
  opts: {
    exts: string[];
    defaultTs?: "bun" | "deno";
    codebases: SyncCodebase[];
  }
): Promise<{ changed: string[]; missing: string[] }> {
  const changed: string[] = [];
  const missing: string[] = [];

  for (const scriptPath of scriptPaths) {
    const localScript = await resolvePreviewLocalScriptState(scriptPath, opts);
    if (!localScript) {
      continue;
    }

    let remoteScript;
    try {
      remoteScript = await wmill.getScriptByPath({
        workspace: workspaceId,
        path: scriptPath,
      });
    } catch {
      missing.push(scriptPath);
      continue;
    }

    const remoteLock = normalizeOptionalString(remoteScript.lock);
    const diverged =
      normalizeComparableContent(localScript.content) !==
        normalizeComparableContent(remoteScript.content) ||
      localScript.language !== remoteScript.language ||
      (localScript.lock !== undefined &&
        normalizeComparableContent(localScript.lock) !==
          normalizeComparableContent(remoteLock)) ||
      localScript.tag !== normalizeOptionalString(remoteScript.tag) ||
      localScript.codebaseDigest !== normalizeOptionalString(remoteScript.codebase);

    if (diverged) {
      changed.push(scriptPath);
    }
  }

  return { changed, missing };
}

function warnAboutLocalPathScriptDivergence(
  divergence: { changed: string[]; missing: string[] }
): void {
  if (divergence.changed.length === 0 && divergence.missing.length === 0) {
    return;
  }

  const details: string[] = [];
  if (divergence.changed.length > 0) {
    details.push(
      `These workspace scripts differ from the deployed version:\n${divergence.changed
        .map((path) => `- ${path}`)
        .join("\n")}`
    );
  }
  if (divergence.missing.length > 0) {
    details.push(
      `These scripts do not exist in the workspace yet:\n${divergence.missing
        .map((path) => `- ${path}`)
        .join("\n")}`
    );
  }

  log.warn(
    `Using local PathScript files for flow preview.\n${details.join("\n")}\nUse --remote to preview deployed workspace scripts instead.`
  );
}

const alreadySynced: string[] = [];

// Collect every script/sub-flow step path in a flow value — recursively through loops,
// branches, and the failure/preprocessor modules — for workspace-path validation. Unlike
// `collectPathScriptPaths` this also includes `type: "flow"` sub-flow steps.
function collectStepPaths(flowValue: any): string[] {
  const paths: string[] = [];
  const walk = (modules: any[] | undefined) => {
    for (const m of modules ?? []) {
      const v = m?.value;
      if (!v) continue;
      if ((v.type === "script" || v.type === "flow") && typeof v.path === "string") {
        paths.push(v.path);
      }
      walk(v.modules);
      walk(v.default);
      for (const b of v.branches ?? []) walk(b?.modules);
      // AI-agent tools are step-like and can carry script paths too.
      walk(v.tools);
    }
  };
  walk(flowValue?.modules);
  if (flowValue?.failure_module) walk([flowValue.failure_module]);
  if (flowValue?.preprocessor_module) walk([flowValue.preprocessor_module]);
  return paths;
}

export async function pushFlow(
  workspace: string,
  remotePath: string,
  localPath: string,
  message?: string,
  permissionedAsContext?: PermissionedAsContext
): Promise<void> {
  if (alreadySynced.includes(localPath)) {
    return;
  }
  alreadySynced.push(localPath);
  remotePath = remotePath.replaceAll(SEP, "/");
  let flow: Flow | undefined = undefined;
  try {
    flow = await wmill.getFlowByPath({
      workspace: workspace,
      path: remotePath,
    });
  } catch {
    // flow doesn't exist
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const localFlow = (await yamlParseFile(localPath + "flow.yaml")) as FlowFile;

  const fileReader = async (path: string) => await readTextFile(localPath + path);
  const missingFiles: string[] = [];
  await replaceInlineScripts(
    localFlow.value.modules,
    fileReader,
    log,
    localPath,
    SEP,
    undefined,
    missingFiles
  );
  if (localFlow.value.failure_module) {
    await replaceInlineScripts([localFlow.value.failure_module], fileReader, log, localPath, SEP, undefined, missingFiles);
  }
  if (localFlow.value.preprocessor_module) {
    await replaceInlineScripts([localFlow.value.preprocessor_module], fileReader, log, localPath, SEP, undefined, missingFiles);
  }
  if (missingFiles.length > 0) {
    // Hard-fail rather than push the literal `!inline path` text as
    // rawscript.content. That string would be persisted in flow_version.value
    // and round-trip as the script body on the next pull, overwriting the
    // user's local handler with the directive — see GIT-871 / #9140.
    throw new Error(
      `Cannot push flow ${remotePath}: missing inline script file(s): ${missingFiles.join(", ")}. ` +
      `Either restore the file(s) or remove the !inline reference(s) from flow.yaml before pushing.`
    );
  }

  // Reject script/sub-flow steps whose path is not a workspace path (u/, f/, g/ or hub/).
  // A flow.yaml generated from a feature-branch checkout can carry absolute local paths
  // (e.g. /tmp/.../ops/scripts/...); pushed, they silently mis-resolve at runtime (#9751).
  // The backend re-validates the same rule for every step type, so this is a fail-fast.
  const badStepPaths = collectStepPaths(localFlow.value).filter(
    (p) => p !== "" && !/^(u|f|g|hub)\//.test(p)
  );
  if (badStepPaths.length > 0) {
    throw new Error(
      `Cannot push flow ${remotePath}: step(s) reference non-workspace path(s): ${badStepPaths.join(", ")}. ` +
      `Flow step paths must be workspace paths (u/, f/, g/ or hub/), not absolute or local filesystem paths. ` +
      `This usually means flow.yaml was generated with paths from a checkout directory.`
    );
  }

  const hasOnBehalfOf = (localFlow as any).has_on_behalf_of ?? !!localFlow.on_behalf_of_email;
  delete (localFlow as any).has_on_behalf_of;

  const preserveFields: { on_behalf_of_email?: string; preserve_on_behalf_of?: boolean } = {};
  if (permissionedAsContext?.userIsAdminOrDeployer && hasOnBehalfOf) {
    if (flow && flow.on_behalf_of_email) {
      preserveFields.on_behalf_of_email = flow.on_behalf_of_email;
      preserveFields.preserve_on_behalf_of = true;
      log.info(`Preserving ${flow.on_behalf_of_email} as on_behalf_of for flow ${remotePath}`);
    }
    // On create: backend applies folder defaults — no client-side resolution needed
  }

  // extra_perms is synced independently via /acls/* (see applyExtraPermsDiff)
  // so a perm-only edit never bumps the flow version. Strip the field from the
  // body that goes to update_flow / create_flow and treat it as a separate
  // step both for the up-to-date short-circuit and after the deploy.
  const { extra_perms: localPerms, ...localFlowBody } = localFlow as FlowFile & {
    extra_perms?: Record<string, boolean>;
  };

  if (flow) {
    if (isSuperset(localFlowBody, flow)) {
      log.info(colors.green(`Flow ${remotePath} is up to date`));
    } else {
      log.info(colors.bold.yellow(`Updating flow ${remotePath}...`));
      await wmill.updateFlow({
        workspace: workspace,
        path: remotePath.replaceAll(SEP, "/"),
        requestBody: {
          path: remotePath.replaceAll(SEP, "/"),
          deployment_message: message,
          ...localFlowBody,
          ...preserveFields,
          // Preserve any user draft at this path (see backend skip_draft_deletion).
          skip_draft_deletion: true,
        },
      });
    }
  } else {
    log.info(colors.bold.yellow("Creating new flow..."));
    try {
      await wmill.createFlow({
        workspace: workspace,
        requestBody: {
          path: remotePath.replaceAll(SEP, "/"),
          deployment_message: message,
          ...localFlowBody,
          ...preserveFields,
          // Preserve any user draft at this path (see backend skip_draft_deletion).
          skip_draft_deletion: true,
        },
      });
    } catch (e) {
      throw new Error(
        //@ts-ignore
        `Failed to create flow ${remotePath}: ${e.body ?? e.message}`
      );
    }
  }

  // Independent of whether the flow body changed, sync extra_perms via /acls/*.
  // Self-contained log line + non-fatal failures.
  //
  // No refetch is needed: extra_perms is item-specific and additive on top of
  // folder perms — folder perms are never merged onto item.extra_perms. And
  // since the request body sent to update_flow / create_flow doesn't carry
  // extra_perms, the value we read in the initial getFlowByPath above is
  // also the post-write value (a no-op deploy can't drift it).
  await applyExtraPermsDiff(
    workspace,
    "flow",
    remotePath.replaceAll(SEP, "/"),
    localPerms,
    (flow as any)?.extra_perms,
  );
}

type Options = GlobalOptions;

async function push(opts: Options & { message?: string }, filePath: string, remotePath: string) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushFlow(workspace.workspaceId, remotePath, filePath, opts.message);
  log.info(colors.bold.underline.green("Flow pushed"));
}

async function list(
  opts: GlobalOptions & { showArchived?: boolean; includeDraftOnly?: boolean; json?: boolean }
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Flow[] = [];
  while (true) {
    const res = await wmill.listFlows({
      workspace: workspace.workspaceId,
      page,
      perPage,
      showArchived: opts.showArchived ?? false,
      includeDraftOnly: opts.includeDraftOnly ?? false,
    });
    page += 1;
    total.push(...res);
    if (res.length < perPage) {
      break;
    }
  }

  if (opts.json) {
    console.log(JSON.stringify(total));
  } else {
    new Table()
      .header(["path", "summary", "edited by"])
      .padding(2)
      .border(true)
      .body(total.map((x) => [x.path, x.summary, x.edited_by]))
      .render();
  }
}
async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const f = await wmill.getFlowByPath({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(f));
  } else {
    console.log(colors.bold("Path:") + " " + f.path);
    console.log(colors.bold("Summary:") + " " + (f.summary ?? ""));
    console.log(colors.bold("Description:") + " " + (f.description ?? ""));
    console.log(colors.bold("Edited by:") + " " + (f.edited_by ?? ""));
    console.log(colors.bold("Edited at:") + " " + (f.edited_at ?? ""));
    // API response type doesn't include flow value/modules — cast needed to access them
    const modules = (f as any).value?.modules;
    if (modules && Array.isArray(modules) && modules.length > 0) {
      console.log(colors.bold("Steps:"));
      function printModules(mods: any[], indent: string = "  ") {
        for (const mod of mods) {
          const type = mod.value?.type ?? "unknown";
          const detail = mod.value?.language ?? mod.value?.path ?? "";
          console.log(`${indent}${mod.id}: ${type}${detail ? " (" + detail + ")" : ""}`);
          if (type === "branchall" || type === "branchone") {
            for (const branch of mod.value?.branches ?? []) {
              console.log(`${indent}  Branch: ${branch.summary || "(default)"}`);
              if (branch.modules) printModules(branch.modules, indent + "    ");
            }
            if (type === "branchone" && mod.value?.default) {
              console.log(`${indent}  Default:`);
              printModules(mod.value.default, indent + "    ");
            }
          } else if (type === "forloopflow" || type === "whileloopflow") {
            if (mod.value?.modules) printModules(mod.value.modules, indent + "  ");
          }
        }
      }
      printModules(modules);
    }
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
    tag?: string;
  },
  path: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};

  // Validate required args against schema when no data provided
  if (!opts.data) {
    try {
      const flow = await wmill.getFlowByPath({
        workspace: workspace.workspaceId,
        path,
      });
      validateRequiredArgs(flow.schema as Record<string, unknown>);
    } catch (e: any) {
      if (e.message?.startsWith("Missing required")) throw e;
      log.warn(`Could not fetch schema to validate args: ${e.message}`);
    }
  }

  const id = await wmill.runFlowByPath({
    workspace: workspace.workspaceId,
    path,
    tag: opts.tag,
    requestBody: input,
  });

  // Build step label map from raw_flow if available
  const stepLabels = new Map<string, string>();
  try {
    const initialJob = await wmill.getJob({
      workspace: workspace.workspaceId,
      id,
    });
    const rawFlow = (initialJob as any).raw_flow;
    if (rawFlow?.modules) {
      for (const mod of rawFlow.modules) {
        if (mod.id) {
          const label = mod.summary ? `${mod.id}: ${mod.summary}` : mod.id;
          stepLabels.set(mod.id, label);
        }
      }
    }
  } catch {
    // Best-effort — fall back to module IDs
  }

  let i = 0;
  let lastStatus = "";
  while (true) {
    const jobInfo = await wmill.getJob({
      workspace: workspace.workspaceId,
      id,
    });

    // Check if flow has completed (success or failure)
    const isCompleted = (jobInfo as any).type === "CompletedJob";
    const flowStatus = jobInfo.flow_status!;

    if (flowStatus.modules.length <= i) {
      break;
    }
    const module = flowStatus.modules[i];

    // If a module has failed, track its job (to show error logs), then break
    if (module.type === "Failure") {
      if (module.job && !opts.silent) {
        const label = stepLabels.get(module.id!) ?? `Step ${i + 1}`;
        log.info("====== " + label + " ======");
        await track_job(workspace.workspaceId, module.job);
      }
      break;
    }

    if (module.job) {
      const label = stepLabels.get(module.id!) ?? `Step ${i + 1}`;
      const isForLoop = (module as any).flow_jobs !== undefined;

      if (isForLoop) {
        // For-loop: track iterations as they appear, re-polling until module completes
        let trackedIterations = 0;
        let forLoopFailed = false;
        while (true) {
          const refreshed = await wmill.getJob({
            workspace: workspace.workspaceId,
            id,
          });
          const refreshedModule = refreshed.flow_status!.modules[i];
          const flowJobs = ((refreshedModule as any).flow_jobs as string[] | undefined) ?? [];

          // Track any new iterations
          while (trackedIterations < flowJobs.length) {
            if (!opts.silent) {
              log.info(`====== ${label} (iteration ${trackedIterations}) ======`);
              await track_job(workspace.workspaceId, flowJobs[trackedIterations]);
            }
            trackedIterations++;
          }

          if (refreshedModule.type === "Success" || refreshedModule.type === "Failure") {
            forLoopFailed = refreshedModule.type === "Failure";
            break;
          }
          await new Promise((resolve) => setTimeout(resolve, 200));
        }
        if (forLoopFailed) break;
      } else {
        if (!opts.silent) {
          log.info("====== " + label + " ======");
          await track_job(workspace.workspaceId, module.job);
        }
      }
    } else {
      // Module not started yet — deduplicate status messages
      const status = String(module.type);
      if (!opts.silent && status !== lastStatus) {
        log.info(colors.dim(status));
        lastStatus = status;
      }
      await new Promise((resolve, _) =>
        setTimeout(() => resolve(undefined), 100)
      );

      // If flow already completed while we were waiting, break out
      if (isCompleted) break;

      continue;
    }
    lastStatus = "";
    i++;
  }

  // Wait for flow completion with retry (handles race when --silent skips module tracking)
  const MAX_RETRIES = 600; // ~60 seconds at 100ms intervals
  let retries = 0;
  while (retries < MAX_RETRIES) {
    try {
      const jobInfo = await wmill.getCompletedJob({
        workspace: workspace.workspaceId,
        id,
      });

      if (!opts.silent) {
        if (jobInfo.success === false) {
          log.info(colors.red.underline.bold("Flow failed"));
        } else {
          log.info(colors.green.underline.bold("Flow ran to completion"));
        }
        log.info("\n");
      }

      if (jobInfo.success === false) {
        process.exitCode = 1;
      }

      if (opts.silent) {
        console.log(JSON.stringify(jobInfo.result ?? {}));
      } else {
        log.info(JSON.stringify(jobInfo.result ?? {}, null, 2));
      }

      break;
    } catch {
      retries++;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  if (retries >= MAX_RETRIES) {
    throw new Error(`Timed out waiting for flow ${id} to complete`);
  }
}

async function preview(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
    remote?: boolean;
    step?: string;
    tag?: string;
  } & SyncOptions,
  flowPath: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  const useLocalPathScripts = !opts.remote;
  if (useLocalPathScripts) {
    opts = await mergeConfigWithConfigFile(opts);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const codebases = useLocalPathScripts ? listSyncCodebases(opts) : [];

  // Normalize path - ensure it's a directory path to a .flow or __flow folder
  const isFlowDir = flowPath.endsWith(".flow") || flowPath.endsWith(".flow" + SEP)
    || flowPath.endsWith("__flow") || flowPath.endsWith("__flow" + SEP);
  if (!isFlowDir) {
    // Check if it's a flow.yaml file
    if (flowPath.endsWith("flow.yaml") || flowPath.endsWith("flow.json")) {
      // Use dirname so a bare "flow.yaml" (no parent dir) becomes "."
      // instead of "" — the latter, after appending SEP below, becomes "/"
      // and silently reads from filesystem root.
      flowPath = dirname(flowPath);
    } else {
      throw new Error(
        "Flow path must be a .flow/__flow directory or a flow.yaml file"
      );
    }
  }

  if (!flowPath.endsWith(SEP)) {
    flowPath += SEP;
  }

  // Read and parse the flow definition
  const localFlow = (await yamlParseFile(flowPath + "flow.yaml")) as FlowFile;

  // Replace inline scripts with their actual content
  const fileReader = async (path: string) => await readTextFile(flowPath + path);
  await replaceInlineScripts(
    localFlow.value.modules,
    fileReader,
    log,
    flowPath,
    SEP
  );
  if (localFlow.value.failure_module) {
    await replaceInlineScripts([localFlow.value.failure_module], fileReader, log, flowPath, SEP);
  }
  if (localFlow.value.preprocessor_module) {
    await replaceInlineScripts([localFlow.value.preprocessor_module], fileReader, log, flowPath, SEP);
  }

  if (useLocalPathScripts) {
    const scriptPaths = collectPathScriptPaths(localFlow.value);
    if (scriptPaths.length > 0) {
      const divergence = await findDivergedLocalPathScripts(
        workspace.workspaceId,
        scriptPaths,
        {
          exts,
          defaultTs: opts.defaultTs,
          codebases,
        }
      );
      if (!opts.silent) {
        warnAboutLocalPathScriptDivergence(divergence);
      }
    }

    const localScriptReader = createPreviewLocalScriptReader({
      exts,
      defaultTs: opts.defaultTs,
      codebases,
    });
    await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
  }

  // Resolve relative imports in inline scripts from local (not-yet-deployed)
  // content so previewing a flow uses locally-edited dependency scripts.
  let tempScriptRefs: Record<string, string> | undefined = undefined;
  if (useLocalPathScripts) {
    const { buildPreviewTempScriptRefs } = await import(
      "../generate-metadata/generate-metadata.ts"
    );
    const resolvedCodebases = (await Promise.resolve(codebases)) as SyncCodebase[];
    tempScriptRefs = await buildPreviewTempScriptRefs(
      workspace,
      opts,
      resolvedCodebases,
      { kind: "flow", folder: flowPath }
    );
  }

  const input = opts.data ? await resolve(opts.data) : {};

  log.debug(`Flow value: ${JSON.stringify(localFlow.value, null, 2)}`);

  // Single-step mode: run only the named module's runnable.
  // The full-flow prep above (inline-script replacement, local PathScript
  // substitution, tempScriptRefs build) is exactly what the single step needs
  // too — PathScript modules have already been rewritten to inline rawscript
  // when `useLocalPathScripts` is set, and tempScriptRefs covers relative
  // imports in inline scripts.
  // Compute the flow's windmill path (e.g. "f/cli_smoke/myrelflow"). Used as
  // the anchor for relative-import resolution: inline scripts in this flow are
  // treated as living at "<flow_wm_path>/<step_id>", so "./util" resolves to
  // "<flow_wm_path_parent>/util" — matching the keys in temp_script_refs.
  const flowWmPath = stripFlowSuffix(flowPath).replaceAll(SEP, "/");

  if (opts.step) {
    await previewStep(opts.step, localFlow, flowWmPath, workspace, input, tempScriptRefs, opts.silent, opts.tag);
    return;
  }

  if (!opts.silent) {
    log.info(colors.yellow(`Running flow preview for ${flowPath}...`));
  }

  // Run the flow preview — start the job, then poll for completion
  const jobId = await wmill.runFlowPreview({
    workspace: workspace.workspaceId,
    requestBody: {
      value: localFlow.value,
      path: flowWmPath,
      args: input,
      tag: opts.tag,
      temp_script_refs: tempScriptRefs,
    },
  });

  const { result, success } = await pollForJobResult(workspace.workspaceId, jobId);

  if (!success) {
    if (opts.silent) {
      console.log(JSON.stringify(result));
    } else {
      log.info(colors.yellow.bold("Flow failed, error handler result:"));
      log.info(JSON.stringify(result, null, 2));
    }
    process.exitCode = 1;
    return;
  }

  if (opts.silent) {
    console.log(JSON.stringify(result));
  } else {
    log.info(colors.bold.underline.green("Flow preview completed"));
    log.info(JSON.stringify(result, null, 2));
  }
}

async function previewStep(
  stepId: string,
  localFlow: FlowFile,
  flowWmPath: string,
  workspace: { workspaceId: string },
  baseArgs: Record<string, unknown>,
  tempScriptRefs: Record<string, string> | undefined,
  silent: boolean,
  tag: string | undefined,
) {
  const module = findStepInFlowValue(localFlow.value, stepId);
  if (!module) {
    const available = collectStepIds(localFlow.value).join(", ") || "(none)";
    throw new Error(`Step '${stepId}' not found in flow. Available steps: ${available}`);
  }

  // The preprocessor module receives args via _ENTRYPOINT_OVERRIDE so the
  // runner picks the preprocessor entrypoint (matches frontend behavior in
  // copilot/chat/flow/core.ts).
  const args =
    stepId === "preprocessor"
      ? { _ENTRYPOINT_OVERRIDE: "preprocessor", ...baseArgs }
      : baseArgs;

  const moduleValue = module.value;
  let jobId: string;
  if (moduleValue?.type === "rawscript") {
    log.info(colors.yellow(`Previewing step '${stepId}' (rawscript, ${moduleValue.language})...`));
    jobId = await wmill.runScriptPreview({
      workspace: workspace.workspaceId,
      requestBody: {
        content: moduleValue.content ?? "",
        language: moduleValue.language,
        // Anchor relative imports to "<flow_wm_path>/<step_id>" so
        // temp_script_refs (keyed by Windmill paths) resolve correctly.
        // Without `path`, the worker defaults to "tmp/main" and "../foo"
        // resolves to "tmp/foo", missing every entry in temp_script_refs.
        path: `${flowWmPath}/${stepId}`,
        flow_path: flowWmPath,
        args,
        tag,
        temp_script_refs: tempScriptRefs,
      },
    });
  } else if (moduleValue?.type === "script") {
    // Falls through here only when the deployed PathScript is what we want —
    // either --remote was passed, or no local file exists for this path.
    log.info(colors.yellow(`Previewing step '${stepId}' (script ${moduleValue.path})...`));
    const script = moduleValue.hash
      ? await wmill.getScriptByHash({
          workspace: workspace.workspaceId,
          hash: moduleValue.hash,
        })
      : await wmill.getScriptByPath({
          workspace: workspace.workspaceId,
          path: moduleValue.path,
        });
    jobId = await wmill.runScriptPreview({
      workspace: workspace.workspaceId,
      requestBody: {
        content: script.content,
        language: script.language as any,
        // Anchor to the script's own deployed path so its relative imports
        // resolve against the workspace tree (or temp_script_refs).
        path: moduleValue.path,
        flow_path: flowWmPath,
        args,
        tag,
        temp_script_refs: tempScriptRefs,
      },
    });
  } else if (moduleValue?.type === "flow") {
    log.info(colors.yellow(`Previewing step '${stepId}' (flow ${moduleValue.path})...`));
    jobId = await wmill.runFlowByPath({
      workspace: workspace.workspaceId,
      path: moduleValue.path,
      tag,
      requestBody: args,
    });
  } else {
    throw new Error(
      `Cannot preview step of type '${moduleValue?.type ?? "unknown"}'. Supported types: rawscript, script, flow.`
    );
  }

  const { result, success } = await pollForJobResult(workspace.workspaceId, jobId);

  if (!success) {
    if (silent) {
      console.log(JSON.stringify(result));
    } else {
      log.info(colors.red.bold(`Step '${stepId}' failed:`));
      log.info(JSON.stringify(result, null, 2));
    }
    process.exitCode = 1;
    return;
  }

  if (silent) {
    console.log(JSON.stringify(result));
  } else {
    log.info(colors.bold.underline.green(`Step '${stepId}' completed`));
    log.info(JSON.stringify(result, null, 2));
  }
}

// Strip the `.flow`/`__flow` directory suffix to recover the flow's logical
// Windmill path. Workspaces with nonDottedPaths use `__flow`; the default
// uses `.flow`. A previous version used `indexOf(".flow")` which returned -1
// (and thus `substring(0, -1) === ""`) for `__flow` folders and for the
// `dirname("flow.yaml") === "."` fallback — producing an empty path that
// broke relative-import resolution downstream.
function stripFlowSuffix(flowPath: string): string {
  const stripped = flowPath.endsWith(SEP) ? flowPath.slice(0, -SEP.length) : flowPath;
  if (stripped.endsWith(".flow")) return stripped.slice(0, -".flow".length);
  if (stripped.endsWith("__flow")) return stripped.slice(0, -"__flow".length);
  return stripped;
}

function findStepInFlowValue(flowValue: any, stepId: string): any | undefined {
  if (!flowValue) return undefined;
  if (flowValue.failure_module?.id === stepId) return flowValue.failure_module;
  if (flowValue.preprocessor_module?.id === stepId) return flowValue.preprocessor_module;
  return findStepInModules(flowValue.modules ?? [], stepId);
}

function findStepInModules(modules: any[], stepId: string): any | undefined {
  for (const m of modules) {
    if (m?.id === stepId) return m;
    const v = m?.value;
    if (!v) continue;
    if (v.type === "forloopflow" || v.type === "whileloopflow") {
      const found = findStepInModules(v.modules ?? [], stepId);
      if (found) return found;
    } else if (v.type === "branchone") {
      for (const b of v.branches ?? []) {
        const found = findStepInModules(b.modules ?? [], stepId);
        if (found) return found;
      }
      const found = findStepInModules(v.default ?? [], stepId);
      if (found) return found;
    } else if (v.type === "branchall") {
      for (const b of v.branches ?? []) {
        const found = findStepInModules(b.modules ?? [], stepId);
        if (found) return found;
      }
    }
  }
  return undefined;
}

function collectStepIds(flowValue: any): string[] {
  const ids: string[] = [];
  const walkModules = (modules: any[]) => {
    for (const m of modules) {
      if (m?.id) ids.push(m.id);
      const v = m?.value;
      if (!v) continue;
      if (v.type === "forloopflow" || v.type === "whileloopflow") {
        walkModules(v.modules ?? []);
      } else if (v.type === "branchone") {
        for (const b of v.branches ?? []) walkModules(b.modules ?? []);
        walkModules(v.default ?? []);
      } else if (v.type === "branchall") {
        for (const b of v.branches ?? []) walkModules(b.modules ?? []);
      }
    }
  };
  if (flowValue?.preprocessor_module?.id) ids.push(flowValue.preprocessor_module.id);
  if (flowValue?.failure_module?.id) ids.push(flowValue.failure_module.id);
  walkModules(flowValue?.modules ?? []);
  return ids;
}

export async function generateLocks(
  opts: GlobalOptions & {
    yes?: boolean;
    dryRun?: boolean;
  } & SyncOptions,
  folder: string | undefined
) {
  log.warn(
    colors.yellow('This command is deprecated. Use "wmill generate-metadata" instead.')
  );
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);
  if (folder) {
    // read script metadata file
    await generateFlowLockInternal(
      folder,
      false,
      workspace,
      opts
    );
  } else {
    const ignore = await ignoreF(opts);
    const elems = Object.keys(
      await elementsToMap(
        await FSFSElement(process.cwd(), [], true),
        (p, isD) => {
          return (
            ignore(p, isD) ||
            (!isD &&
              !p.endsWith(SEP + "flow.yaml") &&
              !p.endsWith(SEP + "flow.json"))
          );
        },
        false,
        {}
      )
    ).map((x) => x.substring(0, x.lastIndexOf(SEP)));
    let hasAny = false;

    for (const folder of elems) {
      const candidate = await generateFlowLockInternal(
        folder,
        true,
        workspace,
        opts
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.yellow.bold(`~ ${candidate}`));
      }
    }

    if (hasAny) {
      if (opts.dryRun) {
        log.info(colors.gray("Dry run complete."));
        return;
      }
      if (
        !opts.yes &&
        !(await Confirm.prompt({
          message: "Update the locks of the inline scripts of the above flows?",
          default: true,
        }))
      ) {
        return;
      }
    } else {
      log.info(colors.green.bold("No locks to update"));
      return;
    }
    for (const folder of elems) {
      await generateFlowLockInternal(
        folder,
        false,
        workspace,
        opts
      );
    }
  }
}

export async function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  flowPath: string
) {
  if (!validatePath(flowPath)) {
    return;
  }

  await loadNonDottedPathsSetting();

  const flowDirFullPath = buildFolderPath(flowPath, "flow");
  mkdirSync(flowDirFullPath, { recursive: true });

  const newFlowDefinition = defaultFlowDefinition();
  if (opts.summary !== undefined) {
    newFlowDefinition.summary = opts.summary;
  }
  if (opts.description !== undefined) {
    newFlowDefinition.description = opts.description;
  }

  const newFlowDefinitionYaml = yamlStringify(
    newFlowDefinition as Record<string, any>
  );

  const metadataFile = getMetadataFileName("flow", "yaml");
  const flowYamlPath = `${flowDirFullPath}/${metadataFile}`;
  writeFileSync(flowYamlPath, newFlowDefinitionYaml, { flag: "wx", encoding: "utf-8" });

  log.info(colors.green(`Created flow at ${flowDirFullPath}`));

  log.info("");
  log.info(colors.bold("To preview this flow:"));
  log.info(colors.gray(`  wmill dev --path ${flowPath}`));
}

async function history(
  opts: GlobalOptions & { json?: boolean },
  flowPath: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const versions = await wmill.getFlowHistory({
    workspace: workspace.workspaceId,
    path: flowPath,
  });

  if (opts.json) {
    console.log(JSON.stringify(versions));
  } else {
    if (versions.length === 0) {
      log.info("No version history found for " + flowPath);
      return;
    }
    new Table()
      .header(["Version", "Created At", "Deployment Message"])
      .padding(2)
      .border(true)
      .body(
        versions.map((v) => [
          String(v.id),
          new Date(v.created_at).toISOString().replace("T", " ").substring(0, 19),
          v.deployment_msg ?? "-",
        ])
      )
      .render();
  }
}

async function showVersion(
  opts: GlobalOptions & { json?: boolean },
  flowPath: string,
  version: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const flow = await wmill.getFlowVersion({
    workspace: workspace.workspaceId,
    path: flowPath,
    version: parseInt(version, 10),
  });

  if (opts.json) {
    console.log(JSON.stringify(flow));
  } else {
    console.log(colors.bold("Path:") + " " + flow.path);
    console.log(colors.bold("Summary:") + " " + (flow.summary ?? "-"));
    console.log(colors.bold("Description:") + " " + (flow.description ?? "-"));
    console.log(colors.bold("Schema:"));
    console.log(JSON.stringify(flow.schema, null, 2));
    console.log(colors.bold("Value:"));
    console.log(JSON.stringify(flow.value, null, 2));
  }
}

const command = new Command()
  .description("flow related commands")
  .option("--show-archived", "Enable archived flows in output")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all flows")
  .option("--show-archived", "Enable archived flows in output")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a flow's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command(
    "push",
    "push a local flow spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .option("--message <message:string>", "Deployment message")
  .action(push as any)
  .command("run", "run a flow by path.")
  .arguments("<path:string>")
  .option(
    "-d --data <data:string>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not ouput anything other then the final output. Useful for scripting."
  )
  .option(
    "--tag <tag:string>",
    "Override the worker tag the run is dispatched to (e.g. to route it to dev workers instead of the flow's default tag)."
  )
  .action(run as any)
  .command(
    "preview",
    "preview a local flow without deploying it. Runs the flow definition from local files and uses local PathScripts by default. Pass --step <id> to run only one module in isolation (resolves nested steps inside branchone/branchall/forloopflow/whileloopflow plus the special preprocessor/failure modules; supported step types: rawscript, script, flow)."
  )
  .arguments("<flow_path:string>")
  .option(
    "-d --data <data:string>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not output anything other then the final output. Useful for scripting."
  )
  .option(
    "--remote",
    "Use deployed workspace scripts for PathScript steps instead of local files."
  )
  .option(
    "--step <step_id:string>",
    "Run only the named step instead of the whole flow. Honors --data as the step's args and --remote / local-PathScript resolution the same way the full-flow preview does."
  )
  .option(
    "--tag <tag:string>",
    "Override the worker tag the preview is dispatched to (e.g. to route it to dev workers instead of the flow's default tag)."
  )
  .action(preview as any)
  .command(
    "generate-locks",
    'DEPRECATED: re-generate flow lock files. Use "wmill generate-metadata" instead.'
  )
  // Deprecated compatibility command. Keep it working for older repos, but
  // exclude it from generated system prompt docs.
  // @deprecated use `wmill generate-metadata`
  .arguments("[flow:file]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account."
  )
  .action(generateLocks as any)
  .command("new", "create a new empty flow")
  .arguments("<flow_path:string>")
  .option("--summary <summary:string>", "flow summary")
  .option("--description <description:string>", "flow description")
  .action(bootstrap as any)
  .command("bootstrap", "create a new empty flow (alias for new)")
  .arguments("<flow_path:string>")
  .option("--summary <summary:string>", "flow summary")
  .option("--description <description:string>", "flow description")
  .action(bootstrap as any)
  .command("history", "Show version history for a flow")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(history as any)
  .command("show-version", "Show a specific version of a flow")
  .arguments("<path:string> <version:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(showVersion as any)
  .command(
    "set-permissioned-as",
    "Set the on_behalf_of_email for a flow (requires admin or wm_deployers group)"
  )
  .arguments("<path:string> <email:string>")
  .action((async (opts: any, flowPath: string, email: string) => {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);
    const remote = await wmill.getFlowByPath({
      workspace: workspace.workspaceId,
      path: flowPath,
    });
    if (!remote) throw new Error(`Flow ${flowPath} not found`);
    await wmill.updateFlow({
      workspace: workspace.workspaceId,
      path: flowPath,
      requestBody: {
        ...remote,
        path: flowPath,
        on_behalf_of_email: email,
        preserve_on_behalf_of: true,
        // Preserve any user draft at this path (see backend skip_draft_deletion).
        skip_draft_deletion: true,
      } as any,
    });
    log.info(colors.green(`Updated permissioned_as for flow ${flowPath} to ${email}`));
  }) as any);

export default command;
