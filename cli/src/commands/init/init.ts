import { stat, writeFile, rm } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Select } from "@cliffy/prompt/select";
import { Input } from "@cliffy/prompt/input";
import * as log from "../../core/log.ts";
import { type WorkspaceBinding } from "./template.ts";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import {
  getActiveWorkspaceOrFallback,
  getActiveWorkspace,
  allWorkspaces,
  add as addWorkspaceProfile,
  type Workspace,
} from "../workspace/workspace.ts";
import { generateRTNamespace } from "../resource-type/resource-type.ts";
import { generateCommentedTemplate } from "./template.ts";
import { refreshPrompts } from "../refresh/prompts.ts";
import { refreshTsconfig } from "../refresh/tsconfig.ts";

export interface InitOptions {
  useDefault?: boolean;
  useBackend?: boolean;
  repository?: string;
  workspace?: string;
  debug?: unknown;
  showDiffs?: boolean;
  token?: string;
  baseUrl?: string;
  configDir?: string;
  bindProfile?: boolean;
}

/**
 * Bootstrap a windmill project with a wmill.yaml file
 */
async function initAction(opts: InitOptions) {
  let didBindWorkspace = false;
  let boundProfile: Workspace | undefined;

  if (await stat("wmill.yaml").catch(() => null)) {
    log.info("wmill.yaml already exists, skipping config generation");
  } else {
    // Detect current git branch for template
    const { isGitRepository, getCurrentGitBranch } = await import(
      "../../utils/git.ts"
    );
    let branchName: string | undefined;
    let wsBindings: WorkspaceBinding[] | undefined;
    // boundProfile is set during bind (hoisted to function scope)
    const inGitRepo = isGitRepository();
    if (inGitRepo) {
      branchName = getCurrentGitBranch() ?? undefined;
    }

    const isInteractive = !!process.stdin.isTTY && !opts.useDefault;

    if (isInteractive && opts.bindProfile !== false) {
      const shouldBind = opts.bindProfile === true || await Confirm.prompt({
        message: "Bind a workspace?",
        default: true,
      });

      if (shouldBind) {
        // Step 1: Pick workspace profile (same as wmill workspace bind)
        let profiles = await allWorkspaces(opts.configDir);
        let selectedProfile: Workspace | undefined;

        if (profiles.length === 0) {
          log.info(colors.yellow("No workspace profiles found. Let's create one."));
          await addWorkspaceProfile(opts as any, undefined, undefined, undefined);
          profiles = await allWorkspaces(opts.configDir);
          selectedProfile = profiles.length > 0
            ? await getActiveWorkspace(opts as GlobalOptions)
            : undefined;
        } else {
          const activeProfile = await getActiveWorkspace(opts as GlobalOptions);
          const orderedProfiles = activeProfile
            ? [
                ...profiles.filter((p) => p.name === activeProfile.name),
                ...profiles.filter((p) => p.name !== activeProfile.name),
              ]
            : profiles;
          const selectedName = await Select.prompt({
            message: "Select workspace profile",
            options: orderedProfiles.map((p) => ({
              name: `${p.name} (${p.workspaceId} on ${p.remote})${
                activeProfile?.name === p.name ? " — active" : ""
              }`,
              value: p.name,
            })),
          });
          selectedProfile = profiles.find((p) => p.name === selectedName);
        }

        if (selectedProfile) {
          // Step 2: Pick workspace name
          const wsName = await Input.prompt({
            message: "Workspace name (key in wmill.yaml)",
            default: selectedProfile.workspaceId,
          });

          // Step 3: Pick git branch (only in git repos)
          let gitBranch: string | undefined;
          if (inGitRepo) {
            const branchInput = await Input.prompt({
              message: "Git branch to associate",
              default: branchName ?? wsName,
            });
            if (branchInput !== wsName) {
              gitBranch = branchInput;
            }
          }

          wsBindings = [{
            name: wsName,
            baseUrl: selectedProfile.remote,
            workspaceId: selectedProfile.workspaceId !== wsName ? selectedProfile.workspaceId : wsName,
            gitBranch,
          }];
          boundProfile = selectedProfile;
        }
      }
    } else if (opts.bindProfile === true) {
      // Non-interactive bind: create a single workspace entry from active profile
      const activeWorkspace = await getActiveWorkspaceOrFallback(
        opts as GlobalOptions
      );
      if (activeWorkspace) {
        const wsName = branchName ?? activeWorkspace.workspaceId;
        wsBindings = [{
          name: wsName,
          baseUrl: activeWorkspace.remote,
          workspaceId: activeWorkspace.workspaceId !== wsName ? activeWorkspace.workspaceId : wsName,
        }];
        boundProfile = activeWorkspace;
      }
    }

    await writeFile("wmill.yaml", generateCommentedTemplate(branchName, undefined, wsBindings), "utf-8");
    log.info(colors.green("wmill.yaml created with default settings"));
    if (wsBindings && wsBindings.length > 0) {
      didBindWorkspace = true;
      log.info(
        colors.green(
          `✓ Bound workspace '${wsBindings[0].name}' → ${wsBindings[0].workspaceId} on ${wsBindings[0].baseUrl}`
        )
      );
      log.info(
        colors.gray("To bind additional workspaces, run: wmill workspace bind")
      );
    } else {
      log.warn(
        "⚠️  No workspace bound. Sync commands will not work without a workspace.\n" +
        "   Run 'wmill workspace bind' to bind a workspace to this project."
      );
    }

    // Create lock file
    await readLockfile();

    // Check for backend git-sync settings — only if a workspace was bound and not --use-default
    if (!opts.useDefault && didBindWorkspace && boundProfile) {
      try {
        const { setClient } = await import("../../core/client.ts");

        // Use the bound profile directly — skip requireLogin which would resolve to the active profile
        setClient(boundProfile.token, boundProfile.remote.replace(/\/$/, ""));

        const wmill = await import("../../../gen/services.gen.ts");
        const settings = await wmill.getSettings({
          workspace: boundProfile.workspaceId,
        });

        if (
          settings.git_sync?.repositories &&
          settings.git_sync.repositories.length > 0
        ) {
          let useBackendSettings = opts.useBackend;

          // If repository is specified, implicitly use backend settings
          if (opts.repository && !opts.useDefault) {
            useBackendSettings = true;
          }

          if (useBackendSettings === undefined) {
            const choice = await Select.prompt({
              message:
                "Git-sync settings found on backend. What would you like to do?",
              options: [
                { name: "Use backend git-sync settings", value: "backend" },
                { name: "Use default settings", value: "default" },
                { name: "Cancel", value: "cancel" },
              ],
            });

            if (choice === "cancel") {
              try {
                await rm("wmill.yaml");
                await rm("wmill-lock.yaml");
              } catch {
                // Ignore cleanup errors
              }
              log.info("Init cancelled");
              process.exit(0);
            }

            useBackendSettings = choice === "backend";
          }

          if (useBackendSettings) {
            log.info("Applying git-sync settings from backend...");
            const { pullGitSyncSettings } = await import(
              "../gitsync-settings/gitsync-settings.ts"
            );
            const gsOpts = {
              ...(opts as GlobalOptions),
              workspace: boundProfile.name,
              repository: opts.repository,
              jsonOutput: false,
              diff: false,
              replace: true,
            };
            (gsOpts as any).__secret_workspace = boundProfile;
            await pullGitSyncSettings(gsOpts);
            log.info(colors.green("Git-sync settings applied from backend"));
          }
        }
      } catch (error) {
        log.warn(
          `Could not check backend for git-sync settings: ${(error as Error).message}`
        );
        log.info("Continuing with default settings");
      }
    }
  }

  await refreshPrompts({ yes: opts.useDefault === true });

  // Generate the IDE tsconfig (managed tsconfig.wmill.json + user tsconfig.json
  // that extends it). Independent of any workspace binding — it's purely local.
  try {
    await refreshTsconfig({ yes: opts.useDefault === true });
  } catch (error) {
    log.warn(
      `Could not generate tsconfig: ${
        error instanceof Error ? error.message : error
      }`
    );
  }

  // Generate resource type namespace (needs a bound workspace)
  if (didBindWorkspace && boundProfile) {
    try {
      // Cache the bound profile so resolveWorkspace doesn't re-resolve and prompt again
      const rtOpts = { ...opts } as GlobalOptions;
      (rtOpts as any).__secret_workspace = boundProfile;
      await generateRTNamespace(rtOpts);
    } catch (error) {
      log.warn(
        `Could not pull resource types and generate TypeScript namespace: ${
          error instanceof Error ? error.message : error
        }`
      );
    }
  } else {
    // generateRTNamespace resolves a workspace; its non-interactive
    // multiple-workspaces path process.exit(-1)s (uncatchable), aborting init.
    // So generate only for a single resolvable workspace (baseUrl + matching
    // profile), passed via __secret_workspace to skip resolution; else skip.
    const { readConfigFile, getWorkspaceNames, getEffectiveWorkspaceId } =
      await import("../../core/conf.ts");
    const config = await readConfigFile({ warnIfMissing: false });
    const resolvable = getWorkspaceNames(config.workspaces).filter(
      (n) => !!(config.workspaces as any)?.[n]?.baseUrl
    );
    let boundProfileForGen: Workspace | undefined;
    if (resolvable.length === 1) {
      const name = resolvable[0];
      const entry = (config.workspaces as any)[name];
      let normalizedBaseUrl: string | undefined;
      try {
        normalizedBaseUrl = new URL(entry.baseUrl).toString();
      } catch {
        normalizedBaseUrl = undefined;
      }
      if (normalizedBaseUrl) {
        const workspaceId = getEffectiveWorkspaceId(name, entry);
        const profiles = await allWorkspaces(opts.configDir);
        boundProfileForGen = profiles.find(
          (p) => p.remote === normalizedBaseUrl && p.workspaceId === workspaceId
        );
      }
    }
    if (boundProfileForGen) {
      try {
        const rtOpts = { ...opts } as GlobalOptions;
        (rtOpts as any).__secret_workspace = boundProfileForGen;
        await generateRTNamespace(rtOpts);
      } catch (error) {
        log.warn(
          `Could not pull resource types and generate TypeScript namespace: ${
            error instanceof Error ? error.message : error
          }`
        );
      }
    } else {
      log.info(
        colors.gray("Skipped resource type namespace generation (no workspace bound). Run 'wmill workspace bind' then 'wmill init' to generate it.")
      );
    }
  }
}

const command = new Command()
  .description("Bootstrap a windmill project with a wmill.yaml file")
  .option("--use-default", "Use default settings without checking backend")
  .option("--use-backend", "Use backend git-sync settings if available")
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when using backend settings"
  )
  .option(
    "--bind-profile",
    "Automatically bind active workspace profile to current Git branch"
  )
  .option("--no-bind-profile", "Skip workspace profile binding prompt")
  .action(initAction as any);

export default command;
