// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "../../types.ts";
import { colors, Command, Input, log, setClient } from "../../../deps.ts";
import { requireLogin } from "../../core/auth.ts";
import { getActiveWorkspace, removeWorkspace } from "./workspace.ts";
import { loginInteractive, tryGetLoginInfo } from "../../core/login.ts";
import * as wmill from "../../../gen/services.gen.ts";

// NOTE: This import will work after regenerating the API client
// Run ./gen_wm_client.sh to regenerate after backend changes
// import * as wmill from "../../../gen/services.gen.ts";

const WM_FORKED_PREFIX = "wm-forked";

async function runGitCommand(
  args: string[],
): Promise<{ success: boolean; output: string }> {
  try {
    const command = new Deno.Command("git", {
      args,
      stdout: "piped",
      stderr: "piped",
    });

    const { code, stdout, stderr } = await command.output();
    const output = new TextDecoder().decode(code === 0 ? stdout : stderr);

    return {
      success: code === 0,
      output: output.trim(),
    };
  } catch (error) {
    return {
      success: false,
      output: `Failed to execute git command: ${error.message}`,
    };
  }
}

async function createWorkspaceFork(
  opts: GlobalOptions,
  workspaceName: string | undefined,
  workspaceId: string | undefined = undefined,
) {
  if (opts.workspace) {
    log.info(
      colors.red.bold(
        "! Workspace needs to be specified as positional argument, not as option."
      )
    );
    return;
  }

  while (workspaceName === undefined) {
    if (!workspaceName) {
      workspaceName = await Input.prompt("Name this forked workspace:");
    }
  }

  if (!workspaceId) {
    workspaceId = await Input.prompt({
      message: "Enter the ID of this workspace, it will also be used as a branch name",
      default: workspaceName,
      suggestions: [workspaceName],
    });
  }

  let token = await tryGetLoginInfo(opts);

  if (!token && Deno.stdin.isTerminal && !Deno.stdin.isTerminal()) {
    log.info("Not a TTY, can't login interactively. Pass the token in --token");
    return;
  }

  const activeWorkspace = await getActiveWorkspace(opts);

  while (!token) {
    token = await loginInteractive(activeWorkspace.remote);
  }
  if (!token) {
    throw new Error("Not logged in. Please run 'wmill workspace add' first.");
  }


  const remote = activeWorkspace.remote
  setClient(
    token,
    remote.endsWith("/") ? remote.substring(0, remote.length - 1) : remote
  );

  log.info(colors.blue(`Creating forked workspace: ${workspaceName}`));

  const trueWorkspaceId = `${WM_FORKED_PREFIX}-${workspaceId}`;
  let alreadyExists = false;
  try {
    alreadyExists = await wmill.existsWorkspace({
      requestBody: { id: trueWorkspaceId },
    });
  } catch (e) {
    log.info(
      colors.red.bold("! Credentials or instance is invalid. Aborting.")
    );
    throw e;
  }

  if (alreadyExists) {
    throw new Error(`This forked workspace '${workspaceId}' (${workspaceName}) already exists. Choose a different id`);
  }

  // Create forked workspace via API
  log.info("Creating forked workspace...");

  try {
    // TODO: Update to createWorkspaceFork after regenerating client from new OpenAPI spec
    const result = await wmill.createWorkspaceFork({
      requestBody: {
        id: trueWorkspaceId,
        name: workspaceName,
        username: undefined, // Let the server handle username
        color: undefined,
        parent_workspace_id: activeWorkspace.workspaceId,
      },
    });

    log.info(colors.green(`✅ ${result}`));

  } catch (error) {
    // If workspace creation fails, we should clean up the git branch
    log.error(
      colors.red(`Failed to create forked workspace: ${error.message}`),
    );
    throw error;
  }

  // Create git branch
  log.info("Creating git branch...");
  const branchResult = await runGitCommand([
    "checkout",
    "-b",
    `${WM_FORKED_PREFIX}/${workspaceId}`,
  ]);
  if (!branchResult.success) {
    // If branch already exists, switch to it
    const switchResult = await runGitCommand(["checkout", `${WM_FORKED_PREFIX}/${workspaceId}`]);
    if (!switchResult.success) {
      throw new Error(
        `Failed to create or switch to git branch: ${branchResult.output}`,
      );
    }
    log.info(colors.yellow(`Switched to existing branch: ${WM_FORKED_PREFIX}/${workspaceId}`));
  } else {
    log.info(colors.green(`Created forked workspace and switched to branch: ${WM_FORKED_PREFIX}/${workspaceId}`));
  }
}

async function deleteWorkspaceFork(
  opts: GlobalOptions & {
    yes?: boolean;
  },
  workspace_id: string,
) {
  if (!workspace_id.startsWith(WM_FORKED_PREFIX)) {
      throw new Error(
        `You can only deleted forked workspaces where the workspace id starts with \`${WM_FORKED_PREFIX}.\` Failed while attempting to delete \`${workspace_id}\``,
      );
  }

  if (!opts.yes) {
        const { Select } = await import("../../../deps.ts");
        const choice = await Select.prompt({
          message: `Are you sure you want to delete the forked workspace with id: \`${workspace_id}\`? This action will delete the workspace `,
          options: [
            { name: "Yes", value: "confirm" },
            { name: "No", value: "cancel" },
          ],
        });

        if (choice === "cancel") {
          log.info("Operation cancelled");
          return;
        }
  }

  log.info(
    colors.green(`✅ Forked workspace '${workspace_id}' deleted successfully!`),
  );
}

const forkCommand = new Command()
  .description("Create a forked workspace and git branch")
  .arguments("<name:string>")
  .option(
    "--create-username <username:string>",
    "Specify your own username in the newly created workspace. Ignored if --create is not specified, the workspace already exists or automatic username creation is enabled on the instance.",
    {
      default: "admin",
    }
  )
  .action(async (opts: GlobalOptions, name: string) => {
    await requireLogin(opts);
    await createWorkspaceFork(opts, name, undefined);
  });

const deleteForkCommand = new Command()
  .description("Delete a forked workspace and git branch")
  .arguments("<forked_workspace_id:string>")
  .action(async (opts: GlobalOptions, name: string) => {
    await deleteWorkspaceFork(opts, name);
  });

export { forkCommand, deleteForkCommand };
