// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { GlobalOptions } from "../../types.ts";
import { colors, Command, log } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import type { ScriptLang } from "../../../gen/types.gen.ts";
import fs from "node:fs";
import { generateHash } from "../../utils/utils.ts";
import { checkifMetadataUptodate, updateMetadataGlobalLock, workspaceDependenciesPathToLanguageAndFilename } from "../../utils/metadata.ts";

async function push(
  opts: GlobalOptions,
  filePath: string,
  language?: ScriptLang,
  name?: string
): Promise<void> {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}`);
  }

  const content = fs.readFileSync(filePath, "utf8");

  // Use the existing pushWorkspaceDependencies function
  await pushWorkspaceDependencies(workspace.workspaceId, filePath, null, content);
}

const command = new Command()
  .alias("deps")
  .description("workspace dependencies related commands")
  .command(
    "push",
    "Push workspace dependencies from a local file"
  )
  .arguments("<file_path:string>")
  .option(
    "--language <language:string>",
    "Programming language (python3, typescript, go, php). If not specified, will be inferred from file extension."
  )
  .option(
    "--name <name:string>",
    "Name for the dependencies. If not specified, creates workspace default dependencies."
  )
  .action(push as any);

export async function pushWorkspaceDependencies(
  workspace: string,
  path: string,
  _befObj: any,
  newDependenciesContent: string
): Promise<void> {
  try {

    let res = workspaceDependenciesPathToLanguageAndFilename(path);
    if (!res) {
      throw new Error(`Unknown workspace dependencies file format: ${path}`);
    }

    let {
      language,
      name
    } = res;

    // TODO: include workspace?
    // Generate hash for workspace dependencies content and metadata
    const contentHash = await generateHash(newDependenciesContent + path);

    // Check if dependencies are up-to-date using wmill-lock.yaml tracking
    const isUpToDate = await checkifMetadataUptodate(path, contentHash, undefined);

    if (isUpToDate) {
      const displayName = name ? `named dependencies "${name}"` : `workspace default dependencies`;
      log.info(colors.green(`${displayName} for ${language} are up-to-date, skipping push`));
      return;
    }

    log.info(colors.yellow(`Pushing ${name ? 'named' : 'workspace default'} dependencies for ${language}...`));

    await wmill.createWorkspaceDependencies({
      workspace,
      requestBody: {
        name,
        content: newDependenciesContent,
        language,
        workspace_id: workspace,
        // Description is not supported in cli, it will use old description
        description: undefined
      }
    });

    // Update wmill-lock.yaml with new hash after successful push
    await updateMetadataGlobalLock(path, contentHash);

    const displayName = name ? `named dependencies "${name}"` : `workspace default dependencies`;
    log.info(colors.green(`Successfully pushed ${displayName} for ${language}`));
  } catch (error: any) {
    log.error(colors.red(`Failed to push workspace dependencies: ${error.message}`));
    throw error;
  }
}

export default command;
