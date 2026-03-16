import { AiAgent, FlowModule, FlowValue, RawScript } from "../gen/types.gen.ts";

export type LocalScriptInfo = {
  content: string;
  language: RawScript["language"];
  lock?: string;
  tag?: string;
};

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

/**
 * Replaces PathScript ("script" type) modules with RawScript ("rawscript" type) using local file content.
 * This is used during flow preview so that local script changes are tested instead of remote versions.
 *
 * @param modules - Array of flow modules to process
 * @param scriptReader - Function that takes a script path and returns local content/language/lock, or undefined if not found locally
 * @param logger - Logger for info/error messages
 */
export async function replacePathScriptsWithLocal(
  modules: FlowModule[],
  scriptReader: (scriptPath: string) => Promise<LocalScriptInfo | undefined>,
  logger: {
    info: (message: string) => void;
    error: (message: string) => void;
  } = {
    info: () => {},
    error: () => {},
  }
): Promise<void> {
  await Promise.all(modules.map(async (module) => {
    if (!module.value) {
      return;
    }

    if (module.value.type === "script") {
      const scriptPath = module.value.path;
      const localScript = await scriptReader(scriptPath);
      if (localScript) {
        logger.info(`Using local script for ${scriptPath}`);
        const pathScript = module.value;
        (module as FlowModule).value = {
          type: "rawscript",
          content: localScript.content,
          language: localScript.language,
          lock: localScript.lock,
          path: scriptPath,
          input_transforms: pathScript.input_transforms,
          tag: pathScript.tag_override ?? localScript.tag,
        } satisfies RawScript;
      }
    } else if (module.value.type === "forloopflow" || module.value.type === "whileloopflow") {
      await replacePathScriptsWithLocal(module.value.modules, scriptReader, logger);
    } else if (module.value.type === "branchall") {
      await Promise.all(module.value.branches.map(async (branch) => {
        await replacePathScriptsWithLocal(branch.modules, scriptReader, logger);
      }));
    } else if (module.value.type === "branchone") {
      await Promise.all(module.value.branches.map(async (branch) => {
        await replacePathScriptsWithLocal(branch.modules, scriptReader, logger);
      }));
      await replacePathScriptsWithLocal(module.value.default, scriptReader, logger);
    } else if (module.value.type === "aiagent") {
      await Promise.all((module.value.tools ?? []).map(async (tool) => {
        const toolValue = tool.value;
        if (!toolValue || toolValue.tool_type !== "flowmodule" || toolValue.type !== "script") {
          return;
        }
        const localScript = await scriptReader(toolValue.path);
        if (localScript) {
          logger.info(`Using local script for AI agent tool ${tool.id}: ${toolValue.path}`);
          (tool as AiAgent["tools"][number]).value = {
            tool_type: "flowmodule",
            type: "rawscript",
            content: localScript.content,
            language: localScript.language,
            lock: localScript.lock,
            path: toolValue.path,
            input_transforms: toolValue.input_transforms,
            tag: toolValue.tag_override ?? localScript.tag,
          };
        }
      }));
    }
  }));
}

/**
 * Replaces all PathScript modules in a flow value (modules, failure_module, preprocessor_module)
 * with RawScript using local file content.
 */
export async function replaceAllPathScriptsWithLocal(
  flowValue: FlowValue,
  scriptReader: (scriptPath: string) => Promise<LocalScriptInfo | undefined>,
  logger: {
    info: (message: string) => void;
    error: (message: string) => void;
  } = {
    info: () => {},
    error: () => {},
  }
): Promise<void> {
  await replacePathScriptsWithLocal(flowValue.modules, scriptReader, logger);
  if (flowValue.failure_module) {
    await replacePathScriptsWithLocal([flowValue.failure_module], scriptReader, logger);
  }
  if (flowValue.preprocessor_module) {
    await replacePathScriptsWithLocal([flowValue.preprocessor_module], scriptReader, logger);
  }
}

/**
 * Creates a scriptReader callback that resolves script paths to local file content.
 */
export function createLocalScriptReader(
  exts: string[],
  inferLanguage: (filePath: string) => RawScript["language"],
  readFile: (path: string) => Promise<string>,
  stat: (path: string) => Promise<{ isFile(): boolean }>,
): (scriptPath: string) => Promise<LocalScriptInfo | undefined> {
  return async (scriptPath) => {
    for (const ext of exts) {
      const filePath = scriptPath + ext;
      try {
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) continue;
        const content = await readFile(filePath);
        const language = inferLanguage(filePath);
        let lock: string | undefined;
        try {
          lock = await readFile(scriptPath + ".script.lock");
        } catch {
          // no lock file
        }
        return { content, language, lock };
      } catch {
        // file doesn't exist with this extension
      }
    }
    return undefined;
  };
}
