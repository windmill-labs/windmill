// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";
import { colors, Command, log } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import fs from "node:fs";
import { workspaceDependenciesPathToLanguageAndFilename } from "../../utils/metadata.ts";

async function push(
  opts: GlobalOptions,
  filePath: string,
): Promise<void> {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`);
  }

  const content = fs.readFileSync(filePath, "utf8");

  // Use the existing pushWorkspaceDependencies function
  await pushWorkspaceDependencies(
    workspace.workspaceId,
    filePath,
    null,
    content,
  );
}

const command = new Command()
  .alias("deps")
  .description("workspace dependencies related commands")
  .command("push", "Push workspace dependencies from a local file")
  .arguments("<file_path:string>")
  .action(push as any);

export async function pushWorkspaceDependencies(
  workspace: string,
  path: string,
  _befObj: any,
  newDependenciesContent: string,
): Promise<void> {
  try {
    const res = workspaceDependenciesPathToLanguageAndFilename(path);
    if (!res) {
      throw new Error(`Unknown workspace dependencies file format: ${path}`);
    }

    const { language, name } = res;

    const displayName = name
      ? `named dependencies "${name}"`
      : `workspace default dependencies`;

    // Fetch remote workspace dependencies and compare content directly
    try {
      const remoteDeps = await wmill.getLatestWorkspaceDependencies({
        workspace,
        language,
        name,
      });

      if (remoteDeps && remoteDeps.content === newDependenciesContent) {
        log.info(
          colors.green(
            `${displayName} for ${language} are up-to-date, skipping push`,
          ),
        );
        return;
      }
    } catch (e: any) {
      // If 404 or not found, the dependency doesn't exist remotely yet - proceed with push
      if (e.status !== 404 && !e.message?.includes("not found")) {
        throw e;
      }
    }

    log.info(
      colors.yellow(
        `Pushing ${
          name ? "named" : "workspace default"
        } dependencies for ${language}...`,
      ),
    );

    await wmill.createWorkspaceDependencies({
      workspace,
      requestBody: {
        name,
        content: newDependenciesContent,
        language,
        workspace_id: workspace,
        // Description is not supported in cli, it will use old description
        description: undefined,
      },
    });

    log.info(
      colors.green(`Successfully pushed ${displayName} for ${language}`),
    );
  } catch (error: any) {
    log.error(
      colors.red(`Failed to push workspace dependencies: ${error.message}`),
    );
    throw error;
  }
}

export default command;
