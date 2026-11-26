import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { markRequestsAsSyncOrigin, setClient } from "../../core/client.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { OpenAPI } from "../../../gen/core/OpenAPI.ts";
import {
  isTriggerOrScheduleKind,
  parseDatatableMigrationDeployPath,
  type DeployKind,
  type TriggerDeployKind,
} from "../../../windmill-utils-internal/src/deploy.ts";
import { offerToRunNewMigrations } from "../datatable_migrations.ts";
import { legacyDelete, legacyDeploy } from "./merge_legacy.ts";

const triggerGetters: Record<
  TriggerDeployKind,
  (p: { workspace: string; path: string }) => Promise<{ permissioned_as?: string }>
> = {
  http_trigger: wmill.getHttpTrigger,
  websocket_trigger: wmill.getWebsocketTrigger,
  kafka_trigger: wmill.getKafkaTrigger,
  nats_trigger: wmill.getNatsTrigger,
  postgres_trigger: wmill.getPostgresTrigger,
  mqtt_trigger: wmill.getMqttTrigger,
  sqs_trigger: wmill.getSqsTrigger,
  gcp_trigger: wmill.getGcpTrigger,
  azure_trigger: wmill.getAzureTrigger,
  email_trigger: wmill.getEmailTrigger,
};

/** Who the item runs as in `workspace`, in the format its kind stores it, if it records one. */
async function getOnBehalfOf(
  kind: DeployKind,
  path: string,
  workspace: string
): Promise<string | undefined> {
  const p = { workspace, path };
  try {
    if (kind === "flow") return (await wmill.getFlowByPath(p)).on_behalf_of_email;
    if (kind === "script") return (await wmill.getScriptByPath(p)).on_behalf_of_email;
    if (kind === "app" || kind === "raw_app") {
      return (await wmill.getAppByPath(p)).policy?.on_behalf_of_email;
    }
    if (kind === "resource") {
      // Only an agent runs as anyone, and like a trigger it records the principal.
      const resource = await wmill.getResource(p);
      return resource.resource_type === "ai_agent"
        ? (resource.value as { on_behalf_of?: string } | undefined)?.on_behalf_of
        : undefined;
    }
    if (kind === "schedule") return (await wmill.getSchedule(p)).permissioned_as;
    if (kind in triggerGetters) {
      return (await triggerGetters[kind as TriggerDeployKind](p)).permissioned_as;
    }
  } catch {
    // Item may not exist
  }
  return undefined;
}

/** Set once the server turns out to have no `deploy_from_workspace`. */
let serverLacksDeploy = false;

/**
 * Deploy one item into `workspaceTo` on the server, or remove it there. A server that predates
 * the server-side deploy gets the client-side one instead.
 */
async function deployFromWorkspace(
  body: Parameters<typeof wmill.deployFromWorkspace>[0]["requestBody"],
  workspaceTo: string
): Promise<{ success: boolean; error?: string }> {
  const kind = body.kind as DeployKind;
  const legacy = () =>
    body.delete
      ? legacyDelete(kind, body.path, workspaceTo)
      : legacyDeploy(kind, body.path, body.source_workspace!, workspaceTo, body.on_behalf_of);
  if (serverLacksDeploy) return legacy();
  try {
    await wmill.deployFromWorkspace({
      workspace: workspaceTo,
      requestBody: {
        ...body,
        // The CLI's remote URL serves both workspaces, so it is where a GCP or Azure push
        // subscription posts. `OpenAPI.BASE` is `<host>/api`; the backend appends its own path.
        push_base_url: OpenAPI.BASE.replace(/\/api\/?$/, ""),
      },
    });
    return { success: true };
  } catch (e) {
    const err = e as { status?: number; body?: string; message?: string };
    // An unknown route answers 404 with no body; a missing item says what is missing.
    if (err.status === 404 && !err.body) {
      serverLacksDeploy = true;
      return legacy();
    }
    return { success: false, error: err.body || err.message || String(e) };
  }
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
      "Could not resolve workspace from branch name. Make sure you are in a git repo with 'workspaces' configured in wmill.yaml."
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
  if (summary.schedules_changed > 0)
    summaryRows.push(["Schedules", String(summary.schedules_changed)]);
  if (summary.triggers_changed > 0)
    summaryRows.push(["Triggers", String(summary.triggers_changed)]);
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
  // Triggers and schedules are opt-in by default — same behavior as the merge
  // UI's `selectDefault`. They often share runtime state with the parent
  // (kafka group_id, postgres replication slot, schedule firing time) and
  // pushing them by default would surprise users running a routine "merge to
  // parent" flow. Override with explicit `--include`, `--all`, or by checking
  // the box in the interactive prompt.
  let selectedDiffs = selectableDiffs;

  if (opts.all) {
    selectedDiffs = selectableDiffs;
  } else if (opts.skipConflicts) {
    // Default-exclude triggers/schedules only when `--include` isn't set —
    // mirrors the `--yes` branch below. With `--include`, the user has
    // explicitly opted into specific items, so the default exclusion would
    // make the include filter return nothing for those kinds.
    selectedDiffs = selectableDiffs.filter(
      (d) =>
        !(d.ahead > 0 && d.behind > 0) &&
        (!!opts.include || !isTriggerOrScheduleKind(d.kind))
    );
  } else if (opts.yes && !opts.include && !opts.exclude) {
    selectedDiffs = selectableDiffs.filter(
      (d) =>
        !isTriggerOrScheduleKind(d.kind) &&
        (direction !== "to-fork" || !(d.ahead > 0 && d.behind > 0))
    );
  } else if (!opts.yes) {
    const { Checkbox } = await import("@cliffy/prompt/checkbox");
    const defaultForToFork = direction === "to-fork";
    const selectedValues = await Checkbox.prompt({
      message: `Select items to deploy (${selectableDiffs.length} available):`,
      options: selectableDiffs.map((d) => {
        const isConflict = d.ahead > 0 && d.behind > 0;
        const isTriggerOrSchedule = isTriggerOrScheduleKind(d.kind);
        const label = `${d.kind}:${d.path}${isConflict ? colors.red(" [CONFLICT]") : ""}`;
        return {
          name: label,
          value: `${d.kind}:${d.path}`,
          checked:
            !isTriggerOrSchedule && (defaultForToFork ? !isConflict : true),
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
  // Updating the fork copies the parent's state in, so nothing it writes there —
  // least of all a deletion — may be read as the fork's own decision. Merging the
  // other way stays authored: someone chose those changes for the target.
  if (direction === "to-fork") {
    markRequestsAsSyncOrigin();
  }
  let successCount = 0;
  let failCount = 0;
  // Datatable migrations deployed (not deleted) into the target. Deploying a
  // migration only upserts its definition — the target schema is unchanged until
  // the migration is run — so offer to run them afterwards (like the push path).
  const deployedMigrations: {
    datatable: string;
    timestamp: number;
    name: string;
  }[] = [];

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
      result = await deployFromWorkspace(
        { kind: diff.kind, path: diff.path, delete: true },
        workspaceTo
      );
    } else {
      const onBehalfOf = opts.preserveOnBehalfOf
        ? await getOnBehalfOf(diff.kind as DeployKind, diff.path, workspaceFrom)
        : undefined;
      result = await deployFromWorkspace(
        {
          kind: diff.kind,
          path: diff.path,
          source_workspace: workspaceFrom,
          on_behalf_of: onBehalfOf,
        },
        workspaceTo
      );
    }

    if (result.success) {
      log.info(colors.green(`  ✓ ${label}`));
      successCount++;
      if (
        !itemDeletedInSource &&
        (diff.kind as DeployKind) === "datatable_migration"
      ) {
        deployedMigrations.push(parseDatatableMigrationDeployPath(diff.path));
      }
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

  // 13. Deployed migration definitions don't touch the target schema until run;
  // offer to run them on the target now (interactive only, like the push path).
  if (deployedMigrations.length > 0) {
    try {
      await offerToRunNewMigrations(workspaceTo, deployedMigrations, {
        yes: opts.yes,
      });
    } catch (e) {
      log.warn(colors.yellow(`Failed to run deployed migrations: ${e}`));
    }
  }
}

export { mergeWorkspaces };
