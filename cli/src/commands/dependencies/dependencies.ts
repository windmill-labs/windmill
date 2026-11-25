import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import * as log from "../../core/log.ts";
import * as wmill from "../../../gen/services.gen.ts";
import fs from "node:fs";
import { workspaceDependenciesPathToLanguageAndFilename } from "../../utils/metadata.ts";
import {
  apiErrorMessage,
  isNoResponseError,
  networkErrorMessage,
  readTextFileSync,
  sleep,
} from "../../utils/utils.ts";

async function push(
  opts: GlobalOptions,
  filePath: string,
): Promise<void> {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`);
  }

  const content = readTextFileSync(filePath);

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

export interface WaitOptions {
  intervalMs: number;
  timeoutMs: number;
}

const DEFAULT_WAIT: WaitOptions = { intervalMs: 10_000, timeoutMs: 30 * 60_000 };

async function waitForWorkspaceDependencies(
  workspace: string,
  language: Parameters<typeof wmill.getLatestWorkspaceDependencies>[0]["language"],
  name: string | undefined,
  content: string,
  pushError: unknown,
  { intervalMs, timeoutMs }: WaitOptions,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await sleep(intervalMs);
    try {
      const latest = await wmill.getLatestWorkspaceDependencies({
        workspace,
        language,
        name,
      });
      if (latest?.content === content) {
        return;
      }
    } catch (e) {
      log.debug(`Still waiting for workspace dependencies: ${e}`);
    }
  }
  throw new Error(
    `The server never applied the pushed ${language} workspace dependencies within ` +
      `${Math.round(timeoutMs / 60_000)} min of the request getting no response. ` +
      `A server older than this CLI cancels the request when the connection drops: ` +
      `retry the push once the server is idle.`,
    { cause: pushError },
  );
}

export async function pushWorkspaceDependencies(
  workspace: string,
  path: string,
  _befObj: any,
  newDependenciesContent: string,
  waitOptions: WaitOptions = DEFAULT_WAIT,
): Promise<void> {
  const res = workspaceDependenciesPathToLanguageAndFilename(path);
  if (!res) {
    throw new Error(
      `Unknown workspace dependencies file format: ${path}. ` +
      `Valid files: package.json, requirements.in, composer.json, go.mod, modules.json`
    );
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

  try {
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
  } catch (e) {
    if (!isNoResponseError(e)) {
      throw e;
    }
    // The first default dependencies of a workspace rebuild its whole dependency map before
    // the server answers, which can outlast this client's or a proxy's timeout. The server
    // finishes regardless, so wait for the pushed content to become the latest version.
    log.info(
      colors.yellow(
        `No response from the server while pushing ${displayName} for ${language} (${
          networkErrorMessage(e) ?? apiErrorMessage(e)
        }). Waiting for the server to apply them...`,
      ),
    );
    await waitForWorkspaceDependencies(
      workspace,
      language,
      name,
      newDependenciesContent,
      e,
      waitOptions,
    );
  }

  log.info(
    colors.green(`Successfully pushed ${displayName} for ${language}`),
  );
}

export default command;
