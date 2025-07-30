import { FlowModule } from "../gen/types.gen.ts";

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
    removeLocks?: string[],
    renamer?: (path: string, newPath: string) => void,
    deleter?: (path: string) => void
  ): Promise<void> {
    await Promise.all(modules.map(async (module) => {
      if (!module.value) {
        throw new Error(`Module value is undefined for module ${module.id}`);
      }
  
      if (module.value.type === "rawscript" && module.value.content && module.value.content.startsWith("!inline")) {
          const path = module.value.content.split(" ")[1];
          try {
            module.value.content = await fileReader(path);
          } catch {
            logger.error(`Script file ${path} not found`);
          }

          const pathPrefix = path.split(".")[0];
          // rename the file if the prefix is different from the module id (fix old naming)
          if (pathPrefix != module.id && renamer) {
            const pathSuffix = path.split(".").slice(1).join(".");
            logger.info(`Renaming ${path} to ${module.id}.${pathSuffix}`);
            renamer(localPath + path, localPath + module.id + "." + pathSuffix);
          }

          const lock = module.value.lock;
          if (removeLocks && removeLocks.includes(path)) {
            module.value.lock = undefined;

          // delete the file if the prefix is different from the module id (fix old naming)
          if (lock && lock != "") {
            const path = lock.split(" ")[1];
            const pathPrefix = path.split(".")[0];
            if (pathPrefix != module.id && deleter) {
              logger.info(`Deleting ${path}`);
              try {
                deleter(localPath + path);
              } catch {
                logger.error(`Failed to delete ${path}`);
              } 
            }
          }

          } else if (
            lock &&
            typeof lock == "string" &&
            lock.trimStart().startsWith("!inline ")
          ) {
            const path = lock.split(" ")[1];
            try {
              module.value.lock = await fileReader(path);
            } catch {
              logger.error(`Lock file ${path} not found`);
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