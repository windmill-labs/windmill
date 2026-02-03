import { FlowModule, RawScript } from "../gen/types.gen.ts";

async function replaceRawscriptInline(
  id: string,
  rawscript: RawScript,
  fileReader: (path: string) => Promise<string>,
  logger: { info: (message: string) => void; error: (message: string) => void },
  separator: string,
  removeLocks?: string[]
): Promise<void> {
  if (!rawscript.content || !rawscript.content.startsWith("!inline")) {
    return;
  }

  const path = rawscript.content.split(" ")[1];
  const pathSuffix = path.split(".").slice(1).join(".");
  const newPath = id + "." + pathSuffix;

  try {
    rawscript.content = await fileReader(path);
  } catch {
    logger.error(`Script file ${path} not found`);
    try {
      rawscript.content = await fileReader(newPath);
    } catch {
      logger.error(`Script file ${newPath} not found`);
    }
  }

  const lock = rawscript.lock;
  if (removeLocks && removeLocks.includes(path)) {
    rawscript.lock = undefined;
  } else if (
    lock &&
    typeof lock === "string" &&
    lock.trimStart().startsWith("!inline ")
  ) {
    const lockPath = lock.split(" ")[1];
    try {
      rawscript.lock = await fileReader(lockPath.replaceAll("/", separator));
    } catch {
      logger.error(`Lock file ${lockPath} not found, treating as empty`);
      rawscript.lock = "";
    }
  }
}

/**
 * Replaces inline script references with actual file content from the filesystem.
 * This function recursively processes all flow modules and their nested structures.
 * 
 * @param modules - Array of flow modules to process
 * @param fileReader - Function to read file content (typically fs.readFile or similar)
 * @param logger - Optional logger object with info and error methods
 * @param localPath - Base path for resolving relative file paths
 * @param removeLocks - Optional array of paths for which to remove lock files
 * @returns Promise that resolves when all inline scripts have been replaced
 */
export async function replaceInlineScripts(
    modules: FlowModule[],
    fileReader: (path: string) => Promise<string>,
    logger: {
      info: (message: string) => void,
      error: (message: string) => void,
    } = {
      info: () => {},
      error: () => {},
    },
    localPath: string,
    separator: string = "/",
    removeLocks?: string[],
    // renamer?: (path: string, newPath: string) => void,
    // deleter?: (path: string) => void
  ): Promise<void> {
    await Promise.all(modules.map(async (module) => {
      if (!module.value) {
        throw new Error(`Module value is undefined for module ${module.id}`);
      }
  
      if (module.value.type === "rawscript") {
        await replaceRawscriptInline(
          module.id,
          module.value,
          fileReader,
          logger,
          separator,
          removeLocks
        );
      } else if (module.value.type === "forloopflow" || module.value.type === "whileloopflow") {
        await replaceInlineScripts(module.value.modules, fileReader, logger, localPath, separator, removeLocks);
      } else if (module.value.type === "branchall") {
        await Promise.all(module.value.branches.map(async (branch) => {
          await replaceInlineScripts(branch.modules, fileReader, logger, localPath, separator, removeLocks);
        }));
      } else if (module.value.type === "branchone") {
        await Promise.all(module.value.branches.map(async (branch) => {
          await replaceInlineScripts(branch.modules, fileReader, logger, localPath, separator, removeLocks);
        }));
        await replaceInlineScripts(module.value.default, fileReader, logger, localPath, separator, removeLocks);
      } else if (module.value.type === "aiagent") {
        await Promise.all((module.value.tools ?? []).map(async (tool) => {
          const toolValue = tool.value;
          if (
            !toolValue ||
            toolValue.tool_type !== "flowmodule" ||
            toolValue.type !== "rawscript"
          ) {
            return;
          }
          await replaceRawscriptInline(
            tool.id,
            toolValue,
            fileReader,
            logger,
            separator,
            removeLocks
          );
        }));
      }
    }));
  }