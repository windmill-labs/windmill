import { stat } from "node:fs/promises";

import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import * as wmill from "../../../gen/services.gen.ts";
import { Resource } from "../../../gen/types.gen.ts";
import { readInlinePathSync } from "../../utils/utils.ts";
import { isBranchSpecificFile } from "../../core/specific_items.ts";
import { getCurrentGitBranch } from "../../utils/git.ts";

export interface ResourceFile {
  value: any;
  description?: string;
  resource_type: string;
  is_oauth?: boolean; // deprecated
}

export async function pushResource(
  workspace: string,
  remotePath: string,
  resource: ResourceFile | Resource | undefined,
  localResource: ResourceFile,
  originalLocalPath?: string
): Promise<void> {
  remotePath = removeType(remotePath, "resource");
  try {
    resource = await wmill.getResource({
      workspace: workspace,
      path: remotePath.replaceAll(SEP, "/"),
    });
  } catch {
    // flow doesn't exist
  }

  // Helper function to resolve inline content
  const resolveInlineContent = async () => {
    if (localResource.value["content"]?.startsWith("!inline ")) {
      const basePath = localResource.value["content"].split(" ")[1];

      // If we're processing a branch-specific metadata file, read from branch-specific resource file

      let pathToRead = basePath;

      if (originalLocalPath && isBranchSpecificFile(originalLocalPath)) {
        const currentBranch = getCurrentGitBranch();
        if (currentBranch) {
          // Directly construct branch-specific resource file path
          const resourcePathSegments = basePath.split(".");
          if (resourcePathSegments.length >= 4 && resourcePathSegments[resourcePathSegments.length - 3] === "resource" && resourcePathSegments[resourcePathSegments.length - 2] === "file") {
            const fileBaseParts = resourcePathSegments.slice(0, -3);
            const fileExt = resourcePathSegments.slice(-3);
            pathToRead = [...fileBaseParts, currentBranch, ...fileExt].join(".");
          }
        }
      }

      localResource.value["content"] = readInlinePathSync(pathToRead);
    }
  };

  if (resource) {
    if (isSuperset(localResource, resource)) {
      return;
    }

    // Only resolve inline content if we're actually updating
    await resolveInlineContent();

    await wmill.updateResource({
      workspace: workspace,
      path: remotePath.replaceAll(SEP, "/"),
      requestBody: { ...localResource },
    });
  } else {
    // New resource - resolve inline content
    await resolveInlineContent();

    if (localResource.is_oauth) {
      log.info(
        colors.yellow(
          "! is_oauth has been removed in newer versions. Ignoring."
        )
      );
    }

    log.info(colors.yellow.bold("Creating new resource..."));
    await wmill.createResource({
      workspace: workspace,
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
        ...localResource,
      },
    });
  }
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  log.info(colors.bold.yellow("Pushing resource..."));

  await pushResource(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath),
    filePath  // Pass the local file path for branch-specific inline content resolution
  );
  log.info(colors.bold.underline.green(`Resource ${remotePath} pushed`));
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  let page = 0;
  const perPage = 10;
  const total: Resource[] = [];
  while (true) {
    const res = await wmill.listResource({
      workspace: workspace.workspaceId,
      page,
      perPage,
    });
    total.push(...res);
    page += 1;
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["Path", "Resource Type"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.resource_type]))
    .render();
}

const command = new Command()
  .description("resource related commands")
  .action(list as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
