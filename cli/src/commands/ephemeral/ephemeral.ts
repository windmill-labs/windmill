// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "../../types.ts";
import { colors, Command, Input, log, setClient } from "../../../deps.ts";
import { requireLogin } from "../../core/auth.ts";
import { getActiveWorkspace, removeWorkspace } from "../workspace/workspace.ts";
import { loginInteractive, tryGetLoginInfo } from "../../core/login.ts";
import * as wmill from "../../../gen/services.gen.ts";

// NOTE: This import will work after regenerating the API client
// Run ./gen_wm_client.sh to regenerate after backend changes
// import * as wmill from "../../../gen/services.gen.ts";

const WM_EPHEMERAL_PREFIX = "wm-ephemeral";

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

async function createEphemeralWorkspace(
  opts: GlobalOptions,
  workspaceName: string | undefined,
  workspaceId: string | undefined,
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
      workspaceName = await Input.prompt("Name this ephemeral workspace:");
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

  log.info(colors.blue(`Creating ephemeral workspace: ${workspaceName}`));

  const trueWorkspaceId = `${WM_EPHEMERAL_PREFIX}-${workspaceId}`;
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
    throw new Error(`This ephemeral workspace '${workspaceId}' (${workspaceName}) already exists. Choose a different id`);
  }

  // Create ephemeral workspace via API
  log.info("Creating ephemeral workspace...");

  try {
    const result = await wmill.createEphemeralWorkspace({
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
      colors.red(`Failed to create ephemeral workspace: ${error.message}`),
    );
    throw error;
  }

  // Create git branch
  log.info("Creating git branch...");
  const branchResult = await runGitCommand([
    "checkout",
    "-b",
    `${WM_EPHEMERAL_PREFIX}/${workspaceId}`,
  ]);
  if (!branchResult.success) {
    // If branch already exists, switch to it
    const switchResult = await runGitCommand(["checkout", `${WM_EPHEMERAL_PREFIX}/${workspaceId}`]);
    if (!switchResult.success) {
      throw new Error(
        `Failed to create or switch to git branch: ${branchResult.output}`,
      );
    }
    log.info(colors.yellow(`Switched to existing branch: ${WM_EPHEMERAL_PREFIX}/${workspaceId}`));
  } else {
    log.info(colors.green(`Created ephemral workspace and switched to branch: ${WM_EPHEMERAL_PREFIX}/${workspaceId}`));
  }
}

async function deleteEphemeralWorkspace(
  opts: GlobalOptions,
  name: string,
) {
  log.info(colors.blue(`Deleting ephemeral workspace: ${name}`));

  // Remove workspace from local config
  log.info("Removing workspace from local configuration...");
  await removeWorkspace(name, false, opts);

  // Switch to main branch and delete ephemeral branch
  log.info("Cleaning up git branch...");

  // Check current branch
  const currentBranchResult = await runGitCommand(["branch", "--show-current"]);
  const currentBranch = currentBranchResult.output;

  // If we're on the ephemeral branch, switch to main
  if (currentBranch === `ephemeral/${name}`) {
    const switchResult = await runGitCommand(["checkout", "main"]);
    if (!switchResult.success) {
      log.warn(
        colors.yellow(
          `Warning: Could not switch to main branch: ${switchResult.output}`,
        ),
      );
    }
  }

  // Delete the ephemeral branch
  const deleteResult = await runGitCommand([
    "branch",
    "-D",
    `ephemeral/${name}`,
  ]);
  if (!deleteResult.success) {
    log.warn(
      colors.yellow(
        `Warning: Could not delete git branch ephemeral/${name}: ${deleteResult.output}`,
      ),
    );
  } else {
    log.info(colors.green(`✅ Deleted git branch: ephemeral/${name}`));
  }

  log.info(
    colors.green(`✅ Ephemeral workspace '${name}' deleted successfully!`),
  );
}

const command = new Command()
  .name("ephemeral")
  .description("Manage ephemeral workspaces with git branches")
  .command(
    "create",
    new Command()
      .description("Create an ephemeral workspace and git branch")
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
        await createEphemeralWorkspace(opts, name);
      }),
  )
  .command(
    "delete",
    new Command()
      .description("Delete an ephemeral workspace and git branch")
      .arguments("<name:string>")
      .action(async (opts: GlobalOptions, name: string) => {
        await deleteEphemeralWorkspace(opts, name);
      }),
  );

export default command;

