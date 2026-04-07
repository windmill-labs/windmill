import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Kind =
  | "script"
  | "flow"
  | "app"
  | "raw_app"
  | "resource"
  | "variable"
  | "resource_type"
  | "folder";

interface DeployResult {
  success: boolean;
  error?: string;
}

// ---------------------------------------------------------------------------
// Flow module helpers (ported from frontend/src/lib/components/flows/flowExplorer.ts)
// ---------------------------------------------------------------------------

// deno-lint-ignore no-explicit-any
function getSubModules(flowModule: any): any[][] {
  const type = flowModule?.value?.type;
  if (type === "forloopflow" || type === "whileloopflow") {
    return [flowModule.value.modules ?? []];
  } else if (type === "branchall") {
    return (flowModule.value.branches ?? []).map(
      // deno-lint-ignore no-explicit-any
      (branch: any) => branch.modules ?? []
    );
  } else if (type === "branchone") {
    return [
      // deno-lint-ignore no-explicit-any
      ...(flowModule.value.branches ?? []).map((b: any) => b.modules ?? []),
      flowModule.value.default ?? [],
    ];
  } else if (type === "aiagent") {
    if (flowModule.value.tools) {
      return [
        flowModule.value.tools
          // deno-lint-ignore no-explicit-any
          .filter((t: any) => t.value?.type === "script" || t.value?.type === "flow")
          // deno-lint-ignore no-explicit-any
          .map((t: any) => ({
            id: t.id,
            value: t.value,
            summary: t.summary,
          })),
      ];
    }
  }
  return [];
}

// deno-lint-ignore no-explicit-any
function getAllSubmodules(flowModule: any): any[] {
  return getSubModules(flowModule)
    .map((modules) =>
      // deno-lint-ignore no-explicit-any
      modules.flatMap((m: any) => [m, ...getAllSubmodules(m)])
    )
    .flat();
}

// deno-lint-ignore no-explicit-any
function getAllModules(flowModules: any[]): any[] {
  return [
    ...flowModules,
    ...flowModules.flatMap((x) => getAllSubmodules(x)),
  ];
}

// ---------------------------------------------------------------------------
// Folder path helper
// ---------------------------------------------------------------------------

function folderName(path: string): string {
  return path.replace(/^f\//, "");
}

// ---------------------------------------------------------------------------
// checkItemExists — ported from frontend/src/lib/utils_workspace_deploy.ts
// ---------------------------------------------------------------------------

async function checkItemExists(
  kind: Kind,
  path: string,
  workspace: string
): Promise<boolean> {
  if (kind === "flow") {
    return wmill.existsFlowByPath({ workspace, path });
  } else if (kind === "script") {
    return wmill.existsScriptByPath({ workspace, path });
  } else if (kind === "app" || kind === "raw_app") {
    return wmill.existsApp({ workspace, path });
  } else if (kind === "variable") {
    return wmill.existsVariable({ workspace, path });
  } else if (kind === "resource") {
    return wmill.existsResource({ workspace, path });
  } else if (kind === "resource_type") {
    return wmill.existsResourceType({ workspace, path });
  } else if (kind === "folder") {
    return wmill.existsFolder({ workspace, name: folderName(path) });
  }
  throw new Error(`Unknown kind: ${kind}`);
}

// ---------------------------------------------------------------------------
// deployItem — ported from frontend/src/lib/utils_workspace_deploy.ts
// ---------------------------------------------------------------------------

async function deployItem(
  kind: Kind,
  path: string,
  workspaceFrom: string,
  workspaceTo: string,
  onBehalfOf?: string
): Promise<DeployResult> {
  const preserveOnBehalfOf = onBehalfOf !== undefined;

  try {
    const alreadyExists = await checkItemExists(kind, path, workspaceTo);

    if (kind === "flow") {
      // deno-lint-ignore no-explicit-any
      const flow = (await wmill.getFlowByPath({ workspace: workspaceFrom, path })) as any;
      // Clear inline script hashes (same as frontend)
      getAllModules(flow.value?.modules ?? []).forEach(
        // deno-lint-ignore no-explicit-any
        (x: any) => {
          if (x.value?.type === "script" && x.value.hash != undefined) {
            x.value.hash = undefined;
          }
        }
      );
      if (alreadyExists) {
        await wmill.updateFlow({
          workspace: workspaceTo,
          path,
          requestBody: {
            ...flow,
            preserve_on_behalf_of: preserveOnBehalfOf,
            on_behalf_of_email: onBehalfOf,
          },
        });
      } else {
        await wmill.createFlow({
          workspace: workspaceTo,
          requestBody: {
            ...flow,
            preserve_on_behalf_of: preserveOnBehalfOf,
            on_behalf_of_email: onBehalfOf,
          },
        });
      }
    } else if (kind === "script") {
      // deno-lint-ignore no-explicit-any
      const script = (await wmill.getScriptByPath({ workspace: workspaceFrom, path })) as any;
      let parentHash: string | undefined;
      if (alreadyExists) {
        // deno-lint-ignore no-explicit-any
        const existing = (await wmill.getScriptByPath({ workspace: workspaceTo, path })) as any;
        parentHash = existing.hash;
      }
      await wmill.createScript({
        workspace: workspaceTo,
        requestBody: {
          ...script,
          lock: script.lock,
          parent_hash: parentHash,
          preserve_on_behalf_of: preserveOnBehalfOf,
          on_behalf_of_email: onBehalfOf,
        },
      });
    } else if (kind === "app" || kind === "raw_app") {
      // deno-lint-ignore no-explicit-any
      const app = (await wmill.getAppByPath({ workspace: workspaceFrom, path })) as any;
      if (alreadyExists) {
        if (app.raw_app) {
          const secret = await wmill.getPublicSecretOfLatestVersionOfApp({
            workspace: workspaceFrom,
            path: app.path,
          });
          const js = await wmill.getRawAppData({
            secretWithExtension: `${secret}.js`,
            workspace: workspaceFrom,
          });
          const css = await wmill.getRawAppData({
            secretWithExtension: `${secret}.css`,
            workspace: workspaceFrom,
          });
          await wmill.updateAppRaw({
            workspace: workspaceTo,
            path,
            formData: {
              app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
              css,
              js,
            },
          });
        } else {
          await wmill.updateApp({
            workspace: workspaceTo,
            path,
            requestBody: {
              ...app,
              preserve_on_behalf_of: preserveOnBehalfOf,
            },
          });
        }
      } else {
        if (app.raw_app) {
          const secret = await wmill.getPublicSecretOfLatestVersionOfApp({
            workspace: workspaceFrom,
            path: app.path,
          });
          const js = await wmill.getRawAppData({
            secretWithExtension: `${secret}.js`,
            workspace: workspaceFrom,
          });
          const css = await wmill.getRawAppData({
            secretWithExtension: `${secret}.css`,
            workspace: workspaceFrom,
          });
          await wmill.createAppRaw({
            workspace: workspaceTo,
            formData: {
              app: { ...app, preserve_on_behalf_of: preserveOnBehalfOf },
              css,
              js,
            },
          });
        } else {
          await wmill.createApp({
            workspace: workspaceTo,
            requestBody: {
              ...app,
              preserve_on_behalf_of: preserveOnBehalfOf,
            },
          });
        }
      }
    } else if (kind === "variable") {
      const variable = await wmill.getVariable({
        workspace: workspaceFrom,
        path,
        decryptSecret: true,
      });
      if (alreadyExists) {
        await wmill.updateVariable({
          workspace: workspaceTo,
          path,
          requestBody: {
            path,
            value: variable.value ?? "",
            is_secret: variable.is_secret,
            description: variable.description ?? "",
          },
          alreadyEncrypted: false,
        });
      } else {
        await wmill.createVariable({
          workspace: workspaceTo,
          requestBody: {
            path,
            value: variable.value ?? "",
            is_secret: variable.is_secret,
            description: variable.description ?? "",
          },
        });
      }
    } else if (kind === "resource") {
      const resource = await wmill.getResource({
        workspace: workspaceFrom,
        path,
      });
      if (alreadyExists) {
        await wmill.updateResource({
          workspace: workspaceTo,
          path,
          requestBody: {
            path,
            value: resource.value ?? "",
            description: resource.description ?? "",
          },
        });
      } else {
        await wmill.createResource({
          workspace: workspaceTo,
          requestBody: {
            path,
            value: resource.value ?? "",
            resource_type: resource.resource_type,
            description: resource.description ?? "",
          },
        });
      }
    } else if (kind === "resource_type") {
      const rt = await wmill.getResourceType({
        workspace: workspaceFrom,
        path,
      });
      if (alreadyExists) {
        await wmill.updateResourceType({
          workspace: workspaceTo,
          path,
          requestBody: {
            schema: rt.schema,
            description: rt.description ?? "",
          },
        });
      } else {
        await wmill.createResourceType({
          workspace: workspaceTo,
          requestBody: {
            name: rt.name,
            schema: rt.schema,
            description: rt.description ?? "",
          },
        });
      }
    } else if (kind === "folder") {
      const name = folderName(path);
      const folder = await wmill.getFolder({ workspace: workspaceFrom, name });
      if (alreadyExists) {
        await wmill.updateFolder({
          workspace: workspaceTo,
          name,
          requestBody: {
            owners: folder.owners,
            // deno-lint-ignore no-explicit-any
            extra_perms: folder.extra_perms as any,
            summary: folder.summary ?? undefined,
          },
        });
      } else {
        await wmill.createFolder({
          workspace: workspaceTo,
          requestBody: {
            name,
            owners: folder.owners,
            // deno-lint-ignore no-explicit-any
            extra_perms: folder.extra_perms as any,
            summary: folder.summary ?? undefined,
          },
        });
      }
    } else {
      throw new Error(`Unknown kind: ${kind}`);
    }

    return { success: true };
  } catch (e: unknown) {
    const err = e as { body?: string; message?: string };
    return { success: false, error: err.body || err.message || String(e) };
  }
}

// ---------------------------------------------------------------------------
// getOnBehalfOf — fetch source item's on_behalf_of value
// ---------------------------------------------------------------------------

async function getOnBehalfOfForItem(
  kind: Kind,
  path: string,
  workspace: string
): Promise<string | undefined> {
  try {
    if (kind === "flow") {
      // deno-lint-ignore no-explicit-any
      const flow = (await wmill.getFlowByPath({ workspace, path })) as any;
      return flow.on_behalf_of_email;
    } else if (kind === "script") {
      // deno-lint-ignore no-explicit-any
      const script = (await wmill.getScriptByPath({ workspace, path })) as any;
      return script.on_behalf_of_email;
    } else if (kind === "app" || kind === "raw_app") {
      // deno-lint-ignore no-explicit-any
      const app = (await wmill.getAppByPath({ workspace, path })) as any;
      return app.policy?.on_behalf_of_email;
    }
  } catch {
    // Item may not exist
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Main merge command
// ---------------------------------------------------------------------------

async function mergeWorkspaces(
  opts: GlobalOptions & {
    direction?: string;
    all?: boolean;
    skipConflicts?: boolean;
    include?: string;
    exclude?: string;
    preserveOnBehalfOf?: boolean;
    yes?: boolean;
  }
): Promise<void> {
  // 1. Resolve fork workspace
  const workspace = await tryResolveBranchWorkspace(opts);
  if (!workspace) {
    throw new Error(
      "Could not resolve workspace from branch name. Make sure you are in a git repo with gitBranches configured."
    );
  }

  const token = workspace.token;
  if (!token) {
    throw new Error("Not logged in. Please run 'wmill workspace add' first.");
  }

  const remote = workspace.remote;
  setClient(
    token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );

  const forkWorkspaceId = workspace.workspaceId;

  // 2. Find parent workspace
  const userWorkspaces = await wmill.listUserWorkspaces();
  const forkEntry = userWorkspaces.workspaces?.find(
    (w) => w.id === forkWorkspaceId
  );

  if (!forkEntry?.parent_workspace_id) {
    throw new Error(
      `Workspace '${forkWorkspaceId}' is not a fork (no parent_workspace_id). ` +
        `You can only merge from a forked workspace.`
    );
  }

  const parentWorkspaceId = forkEntry.parent_workspace_id;
  log.info(
    `Fork: ${colors.bold(forkWorkspaceId)} → Parent: ${colors.bold(parentWorkspaceId)}`
  );

  // 3. Compare workspaces
  log.info("Comparing workspaces...");
  const comparison = await wmill.compareWorkspaces({
    workspace: parentWorkspaceId,
    targetWorkspaceId: forkWorkspaceId,
  });

  if (comparison.skipped_comparison) {
    log.info(
      colors.yellow(
        "This fork was created before change tracking was available. " +
          "Use the UI or git-based merge instead."
      )
    );
    return;
  }

  const summary = comparison.summary;
  if (summary.total_diffs === 0) {
    log.info(colors.green("Everything is up to date. No differences found."));
    return;
  }

  // 4. Display summary
  log.info("");
  log.info(colors.bold("Comparison Summary:"));

  const summaryRows: string[][] = [];
  if (summary.scripts_changed > 0)
    summaryRows.push(["Scripts", String(summary.scripts_changed)]);
  if (summary.flows_changed > 0)
    summaryRows.push(["Flows", String(summary.flows_changed)]);
  if (summary.apps_changed > 0)
    summaryRows.push(["Apps", String(summary.apps_changed)]);
  if (summary.resources_changed > 0)
    summaryRows.push(["Resources", String(summary.resources_changed)]);
  if (summary.variables_changed > 0)
    summaryRows.push(["Variables", String(summary.variables_changed)]);
  if (summary.resource_types_changed > 0)
    summaryRows.push(["Resource Types", String(summary.resource_types_changed)]);
  if (summary.folders_changed > 0)
    summaryRows.push(["Folders", String(summary.folders_changed)]);
  summaryRows.push(["Total", String(summary.total_diffs)]);
  if (summary.conflicts > 0)
    summaryRows.push([
      colors.red("Conflicts"),
      colors.red(String(summary.conflicts)),
    ]);

  new Table()
    .header(["Type", "Changed"])
    .padding(2)
    .border(true)
    .body(summaryRows)
    .render();

  // 5. Display diffs table
  const diffs = comparison.diffs.filter(
    (d) => d.has_changes !== false
  );
  if (diffs.length === 0) {
    log.info(colors.green("No effective changes to deploy."));
    return;
  }

  log.info("");
  log.info(colors.bold("Changed items:"));

  new Table()
    .header(["#", "Kind", "Path", "Ahead", "Behind", "Conflict"])
    .padding(1)
    .border(true)
    .body(
      diffs.map((d, i) => {
        const isConflict = d.ahead > 0 && d.behind > 0;
        return [
          String(i + 1),
          d.kind,
          d.path,
          d.ahead > 0 ? colors.green(String(d.ahead)) : "0",
          d.behind > 0 ? colors.yellow(String(d.behind)) : "0",
          isConflict ? colors.red("YES") : "",
        ];
      })
    )
    .render();

  // 6. Determine direction
  let direction: "to-parent" | "to-fork";
  if (opts.direction === "to-parent" || opts.direction === "to-fork") {
    direction = opts.direction;
  } else if (opts.direction) {
    throw new Error(
      `Invalid direction '${opts.direction}'. Use 'to-parent' or 'to-fork'.`
    );
  } else if (opts.yes) {
    direction = "to-parent";
  } else {
    const { Select } = await import("@cliffy/prompt/select");
    direction = (await Select.prompt({
      message: "Deploy direction:",
      options: [
        {
          name: `Deploy to parent (${parentWorkspaceId}) ← fork changes`,
          value: "to-parent",
        },
        {
          name: `Update fork (${forkWorkspaceId}) ← parent changes`,
          value: "to-fork",
        },
      ],
    })) as "to-parent" | "to-fork";
  }

  log.info(
    `\nDirection: ${colors.bold(direction === "to-parent" ? `Fork → Parent (${parentWorkspaceId})` : `Parent → Fork (${forkWorkspaceId})`)}`
  );

  // 7. Filter selectable diffs based on direction
  const selectableDiffs = diffs.filter((d) => {
    if (direction === "to-parent") {
      return d.ahead > 0 && (d.exists_in_fork || d.exists_in_source);
    } else {
      return d.behind > 0 && (d.exists_in_source || d.exists_in_fork);
    }
  });

  if (selectableDiffs.length === 0) {
    log.info(
      colors.yellow(
        `No items to deploy in the '${direction}' direction.`
      )
    );
    return;
  }

  // 8. Select items
  let selectedDiffs = selectableDiffs;

  if (opts.all) {
    // Select everything
    selectedDiffs = selectableDiffs;
  } else if (opts.skipConflicts) {
    selectedDiffs = selectableDiffs.filter(
      (d) => !(d.ahead > 0 && d.behind > 0)
    );
  } else if (opts.yes && !opts.include && !opts.exclude) {
    // Non-interactive default: all for to-parent, non-conflicts for to-fork
    if (direction === "to-fork") {
      selectedDiffs = selectableDiffs.filter(
        (d) => !(d.ahead > 0 && d.behind > 0)
      );
    }
  } else if (!opts.yes) {
    // Interactive selection
    const { Checkbox } = await import("@cliffy/prompt/checkbox");
    const defaultForToFork = direction === "to-fork";
    const selectedValues = await Checkbox.prompt({
      message: `Select items to deploy (${selectableDiffs.length} available):`,
      options: selectableDiffs.map((d) => {
        const isConflict = d.ahead > 0 && d.behind > 0;
        const label = `${d.kind}:${d.path}${isConflict ? colors.red(" [CONFLICT]") : ""}`;
        return {
          name: label,
          value: `${d.kind}:${d.path}`,
          checked: defaultForToFork
            ? !isConflict
            : true,
        };
      }),
    });
    selectedDiffs = selectableDiffs.filter((d) =>
      selectedValues.includes(`${d.kind}:${d.path}`)
    );
  }

  // Apply --include filter
  if (opts.include) {
    const includeSet = new Set(opts.include.split(",").map((s) => s.trim()));
    selectedDiffs = selectedDiffs.filter((d) =>
      includeSet.has(`${d.kind}:${d.path}`)
    );
  }

  // Apply --exclude filter
  if (opts.exclude) {
    const excludeSet = new Set(opts.exclude.split(",").map((s) => s.trim()));
    selectedDiffs = selectedDiffs.filter(
      (d) => !excludeSet.has(`${d.kind}:${d.path}`)
    );
  }

  if (selectedDiffs.length === 0) {
    log.info(colors.yellow("No items selected for deployment."));
    return;
  }

  // Warn about conflicts
  const conflicts = selectedDiffs.filter(
    (d) => d.ahead > 0 && d.behind > 0
  );
  if (conflicts.length > 0) {
    log.info(
      colors.yellow(
        `\n⚠ ${conflicts.length} conflicting item(s) will be deployed (source will overwrite target):`
      )
    );
    for (const c of conflicts) {
      log.info(colors.yellow(`  - ${c.kind}:${c.path}`));
    }
    if (!opts.yes) {
      const { Confirm } = await import("@cliffy/prompt/confirm");
      const proceed = await Confirm.prompt(
        "Proceed with deploying conflicting items?"
      );
      if (!proceed) {
        log.info("Aborted.");
        return;
      }
    }
  }

  log.info(
    `\nDeploying ${colors.bold(String(selectedDiffs.length))} item(s)...`
  );

  // 9. Sort: folders first (cast to string — backend can return "folder" even though
  // the generated type union doesn't include it)
  const sorted = [...selectedDiffs].sort((a, b) => {
    const aFolder = (a.kind as string) === "folder" ? 0 : 1;
    const bFolder = (b.kind as string) === "folder" ? 0 : 1;
    return aFolder - bFolder;
  });

  // Determine workspaceFrom and workspaceTo based on direction
  const workspaceFrom =
    direction === "to-parent" ? forkWorkspaceId : parentWorkspaceId;
  const workspaceTo =
    direction === "to-parent" ? parentWorkspaceId : forkWorkspaceId;

  // 10. Deploy
  let successCount = 0;
  let failCount = 0;

  for (const diff of sorted) {
    const label = `${diff.kind}:${diff.path}`;

    // Resolve on_behalf_of if requested
    let onBehalfOf: string | undefined;
    if (opts.preserveOnBehalfOf) {
      onBehalfOf = await getOnBehalfOfForItem(
        diff.kind as Kind,
        diff.path,
        workspaceFrom
      );
    }

    const result = await deployItem(
      diff.kind as Kind,
      diff.path,
      workspaceFrom,
      workspaceTo,
      onBehalfOf
    );

    if (result.success) {
      log.info(colors.green(`  ✓ ${label}`));
      successCount++;
    } else {
      log.info(colors.red(`  ✗ ${label}: ${result.error}`));
      failCount++;
    }
  }

  // 11. Reset diff tally
  try {
    await wmill.resetDiffTally({
      workspace: parentWorkspaceId,
      forkWorkspaceId: forkWorkspaceId,
    });
  } catch {
    // Non-critical — tally will refresh on next comparison
  }

  // 12. Summary
  log.info("");
  if (failCount === 0) {
    log.info(
      colors.green(
        `✅ Successfully deployed ${successCount} item(s) from ${workspaceFrom} to ${workspaceTo}.`
      )
    );
  } else {
    log.info(
      colors.yellow(
        `Deployed ${successCount} item(s), ${colors.red(String(failCount) + " failed")} from ${workspaceFrom} to ${workspaceTo}.`
      )
    );
  }
}

export { mergeWorkspaces };
