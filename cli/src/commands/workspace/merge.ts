import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { setClient } from "../../core/client.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  deployItem,
  deleteItemInWorkspace,
  getOnBehalfOf,
  type DeployKind,
  type DeployProvider,
} from "../../../windmill-utils-internal/src/deploy.ts";

// ---------------------------------------------------------------------------
// Provider adapter — wraps CLI's standalone API functions
// ---------------------------------------------------------------------------

const provider: DeployProvider = {
  existsFlowByPath: wmill.existsFlowByPath,
  existsScriptByPath: wmill.existsScriptByPath,
  existsApp: wmill.existsApp,
  existsVariable: wmill.existsVariable,
  existsResource: wmill.existsResource,
  existsResourceType: wmill.existsResourceType,
  existsFolder: wmill.existsFolder,
  getFlowByPath: wmill.getFlowByPath,
  createFlow: wmill.createFlow,
  updateFlow: wmill.updateFlow,
  archiveFlowByPath: wmill.archiveFlowByPath,
  getScriptByPath: wmill.getScriptByPath,
  createScript: wmill.createScript,
  archiveScriptByPath: wmill.archiveScriptByPath,
  getAppByPath: wmill.getAppByPath,
  createApp: wmill.createApp,
  updateApp: wmill.updateApp,
  createAppRaw: wmill.createAppRaw,
  updateAppRaw: wmill.updateAppRaw,
  getPublicSecretOfLatestVersionOfApp:
    wmill.getPublicSecretOfLatestVersionOfApp,
  getRawAppData: wmill.getRawAppData,
  deleteApp: wmill.deleteApp,
  getVariable: wmill.getVariable,
  createVariable: wmill.createVariable,
  updateVariable: wmill.updateVariable,
  deleteVariable: wmill.deleteVariable,
  getResource: wmill.getResource,
  createResource: wmill.createResource,
  updateResource: wmill.updateResource,
  deleteResource: wmill.deleteResource,
  getResourceType: wmill.getResourceType,
  createResourceType: wmill.createResourceType,
  updateResourceType: wmill.updateResourceType,
  deleteResourceType: wmill.deleteResourceType,
  getFolder: wmill.getFolder,
  createFolder: wmill.createFolder,
  updateFolder: wmill.updateFolder,
  deleteFolder: wmill.deleteFolder,
};

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
  const diffs = comparison.diffs.filter((d) => d.has_changes !== false);
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
      return d.ahead > 0;
    } else {
      return d.behind > 0;
    }
  });

  if (selectableDiffs.length === 0) {
    log.info(
      colors.yellow(`No items to deploy in the '${direction}' direction.`)
    );
    return;
  }

  // 8. Select items
  let selectedDiffs = selectableDiffs;

  if (opts.all) {
    selectedDiffs = selectableDiffs;
  } else if (opts.skipConflicts) {
    selectedDiffs = selectableDiffs.filter(
      (d) => !(d.ahead > 0 && d.behind > 0)
    );
  } else if (opts.yes && !opts.include && !opts.exclude) {
    if (direction === "to-fork") {
      selectedDiffs = selectableDiffs.filter(
        (d) => !(d.ahead > 0 && d.behind > 0)
      );
    }
  } else if (!opts.yes) {
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
          checked: defaultForToFork ? !isConflict : true,
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

  // 9. Sort: folders first
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

    // Check if the item was deleted in the source workspace
    const itemDeletedInSource =
      direction === "to-parent"
        ? diff.exists_in_fork === false
        : diff.exists_in_source === false;

    let result;
    if (itemDeletedInSource) {
      log.info(colors.yellow(`  ⌫ ${label} (removing from target)`));
      result = await deleteItemInWorkspace(
        provider,
        diff.kind as DeployKind,
        diff.path,
        workspaceTo
      );
    } else {
      let onBehalfOf: string | undefined;
      if (opts.preserveOnBehalfOf) {
        onBehalfOf = await getOnBehalfOf(
          provider,
          diff.kind as DeployKind,
          diff.path,
          workspaceFrom
        );
      }

      result = await deployItem(
        provider,
        diff.kind as DeployKind,
        diff.path,
        workspaceFrom,
        workspaceTo,
        onBehalfOf
      );
    }

    if (result.success) {
      log.info(colors.green(`  ✓ ${label}`));
      successCount++;
    } else {
      log.info(colors.red(`  ✗ ${label}: ${result.error}`));
      failCount++;
    }
  }

  // 11. Reset diff tally (only if items were successfully deployed)
  if (successCount > 0) {
    try {
      await wmill.resetDiffTally({
        workspace: parentWorkspaceId,
        forkWorkspaceId: forkWorkspaceId,
      });
    } catch {
      // Non-critical
    }
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
