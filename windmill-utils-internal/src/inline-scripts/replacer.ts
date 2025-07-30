import { FlowModule } from "windmill-client";

/**
 * Replaces inline script references with actual file content from the filesystem.
 * Recursively processes nested modules in loops and branches.
 * 
 * @param modules - Array of flow modules to process
 * @param fileReader - Function to read file content (e.g., fs.readFile)
 * @param logger - Logger object with info and error methods
 * @param localPath - Base path for resolving relative file paths
 * @param removeLocks - Optional array of paths for which to remove lock files
 */
export async function replaceInlineScripts(
    modules: FlowModule[],
    fileReader: (...args: any[]) => any,
    logger: {
      info: (...args: any[]) => void,
      error: (...args: any[]) => void,
    } = {
      info: () => {},
      error: () => {},
    },
    localPath: string,
    removeLocks?: string[]
  ): Promise<void> {
    await Promise.all(modules.map(async (module) => {
      if (!module.value) {
        throw new Error(`Module value is undefined for module ${module.id}`);
      }
  
      if (module.value.type === "rawscript") {
        if (module.value.content.startsWith("!inline")) {
          const path = module.value.content.split(" ")[1];
          try {
            module.value.content = await fileReader(path);
            logger.info(`Replaced inline script ${path} with ${module.value.content}`);
          } catch {
            logger.error(`Script file ${path} not found`);
          }
          const lock = module.value.lock;
          if (removeLocks && removeLocks.includes(path)) {
            module.value.lock = undefined;
          } else if (
            lock &&
            typeof lock == "string" &&
            lock.trimStart().startsWith("!inline ")
          ) {
            const path = lock.split(" ")[1];
            try {
              module.value.lock = await fileReader(path);
              logger.info(`Replaced inline lock ${path} with ${module.value.lock}`);
            } catch {
              logger.error(`Lock file ${path} not found`);
            }
          }
        }
      } else if (module.value.type === "forloopflow" || module.value.type === "whileloopflow") {
        await replaceInlineScripts(module.value.modules, fileReader, logger, localPath, removeLocks);
      } else if (module.value.type === "branchall") {
        await Promise.all(module.value.branches.map(async (branch) => {
          await replaceInlineScripts(branch.modules, fileReader, logger, localPath, removeLocks);
        }));
      } else if (module.value.type === "branchone") {
        await Promise.all(module.value.branches.map(async (branch) => {
          await replaceInlineScripts(branch.modules, fileReader, logger, localPath, removeLocks);
        }));
        await replaceInlineScripts(module.value.default, fileReader, logger, localPath, removeLocks);
      }
    }));
  }