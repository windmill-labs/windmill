// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  Confirm,
  JobService,
  log,
  NewScript,
  readAll,
  Script,
  ScriptService,
  Table,
  writeAllSync,
  yamlStringify,
} from "./deps.ts";
import { deepEqual } from "./utils.ts";
import {
  defaultScriptMetadata,
  scriptBootstrapCode,
} from "./bootstrap/script_bootstrap.ts";

import { Workspace } from "./workspace.ts";
import { generateMetadataInternal, parseMetadataFile } from "./metadata.ts";
import {
  ScriptLanguage,
  inferContentTypeFromFilePath,
} from "./script_common.ts";
import {
  elementsToMap,
  readDirRecursiveWithIgnore,
  yamlOptions,
} from "./sync.ts";
import { ignoreF } from "./sync.ts";
import { FSFSElement } from "./sync.ts";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
  readConfigFile,
} from "./conf.ts";

export interface ScriptFile {
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);

  if (!validatePath(filePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot push a script metadata file, point to the script content file instead (.py, .ts, .go|.sh)"
    );
  }

  await requireLogin(opts);
  const globalDeps = await findGlobalDeps();
  await handleFile(filePath, workspace, [], undefined, opts, globalDeps);
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function handleScriptMetadata(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  globalDeps: GlobalDeps
): Promise<boolean> {
  if (path.endsWith(".script.json") || path.endsWith(".script.yaml")) {
    const contentPath = await findContentFile(path);
    return handleFile(
      contentPath,
      workspace,
      alreadySynced,
      message,
      undefined,
      globalDeps
    );
  } else {
    return false;
  }
}

export async function handleFile(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  opts: (GlobalOptions & { defaultTs?: "bun" | "deno" }) | undefined,
  globalDeps: GlobalDeps
): Promise<boolean> {
  if (
    !path.includes(".inline_script.") &&
    exts.some((exts) => path.endsWith(exts))
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    log.debug(`Processing local script ${path}`);

    alreadySynced.push(path);
    const remotePath = path
      .substring(0, path.indexOf("."))
      .replaceAll("\\", "/");
    const typed = (
      await parseMetadataFile(
        remotePath,
        opts ? { ...opts, path, workspaceRemote: workspace } : undefined,
        globalDeps
      )
    )?.payload;
    const language = inferContentTypeFromFilePath(path, opts?.defaultTs);

    const workspaceId = workspace.workspaceId;

    let remote = undefined;
    try {
      remote = await ScriptService.getScriptByPath({
        workspace: workspaceId,
        path: remotePath.replaceAll("\\", "/"),
      });
      log.debug(`Script ${remotePath} exists on remote`);
    } catch {
      log.debug(`Script ${remotePath} does not exist on remote`);
    }
    const content = await Deno.readTextFile(path);

    if (remote) {
      if (content === remote.content) {
        if (
          typed == undefined ||
          (typed.description === remote.description &&
            typed.summary === remote.summary &&
            typed.kind == remote.kind &&
            !remote.archived &&
            (Array.isArray(remote?.lock)
              ? remote?.lock?.join("\n")
              : remote?.lock ?? ""
            ).trim() == (typed?.lock ?? "").trim() &&
            deepEqual(typed.schema, remote.schema) &&
            typed.tag == remote.tag &&
            (typed.ws_error_handler_muted ?? false) ==
              remote.ws_error_handler_muted &&
            typed.dedicated_worker == remote.dedicated_worker &&
            typed.cache_ttl == remote.cache_ttl &&
            typed.concurrency_time_window_s ==
              remote.concurrency_time_window_s &&
            typed.concurrent_limit == remote.concurrent_limit &&
            Boolean(typed.restart_unless_cancelled) ==
              Boolean(remote.restart_unless_cancelled))
        ) {
          log.info(colors.green(`Script ${remotePath} is up to date`));
          return true;
        }
      }

      log.info(
        colors.yellow.bold(`Creating script with a parent ${remotePath}`)
      );
      await ScriptService.createScript({
        workspace: workspaceId,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          kind: typed?.kind,
          lock: typed?.lock,
          parent_hash: remote.hash,
          schema: typed?.schema,
          tag: typed?.tag,
          ws_error_handler_muted: typed?.ws_error_handler_muted,
          dedicated_worker: typed?.dedicated_worker,
          cache_ttl: typed?.cache_ttl,
          concurrency_time_window_s: typed?.concurrency_time_window_s,
          concurrent_limit: typed?.concurrent_limit,
          deployment_message: message,
          restart_unless_cancelled: typed?.restart_unless_cancelled,
        },
      });
    } else {
      log.info(
        colors.yellow.bold(`Creating script without parent ${remotePath}`)
      );
      // no parent hash
      await ScriptService.createScript({
        workspace: workspaceId,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          kind: typed?.kind,
          lock: typed?.lock,
          parent_hash: undefined,
          schema: typed?.schema,
          tag: typed?.tag,
          ws_error_handler_muted: typed?.ws_error_handler_muted,
          dedicated_worker: typed?.dedicated_worker,
          cache_ttl: typed?.cache_ttl,
          concurrency_time_window_s: typed?.concurrency_time_window_s,
          concurrent_limit: typed?.concurrent_limit,
          deployment_message: message,
          restart_unless_cancelled: typed?.restart_unless_cancelled,
        },
      });
    }
    return true;
  }
  return false;
}

export async function findContentFile(filePath: string) {
  const candidates = filePath.endsWith("script.json")
    ? exts.map((x) => filePath.replace(".script.json", x))
    : exts.map((x) => filePath.replace(".script.yaml", x));

  const validCandidates = (
    await Promise.all(
      candidates.map((x) => {
        return Deno.stat(x)
          .catch(() => undefined)
          .then((x) => x?.isFile)
          .then((e) => {
            return { path: x, file: e };
          });
      })
    )
  )
    .filter((x) => x.file)
    .map((x) => x.path);
  if (validCandidates.length > 1) {
    throw new Error(
      "No content path given and more than one candidate found: " +
        validCandidates.join(", ")
    );
  }
  if (validCandidates.length < 1) {
    throw new Error(
      `No content path given and no content file found for ${filePath}.`
    );
  }
  return validCandidates[0];
}

export function filePathExtensionFromContentType(
  language: ScriptLanguage,
  defaultTs: "bun" | "deno" | undefined
): string {
  if (language === "python3") {
    return ".py";
  } else if (language === "nativets") {
    return ".fetch.ts";
  } else if (language === "bun") {
    if (defaultTs == undefined || defaultTs == "deno") {
      return ".bun.ts";
    } else {
      return ".ts";
    }
  } else if (language === "deno") {
    if (defaultTs == "bun") {
      return ".deno.ts";
    } else {
      return ".ts";
    }
  } else if (language === "go") {
    return ".go";
  } else if (language === "mysql") {
    return ".my.sql";
  } else if (language === "bigquery") {
    return ".bq.sql";
  } else if (language === "snowflake") {
    return ".sf.sql";
  } else if (language === "mssql") {
    return ".ms.sql";
  } else if (language === "postgresql") {
    return ".pg.sql";
  } else if (language === "graphql") {
    return ".gql";
  } else if (language === "bash") {
    return ".sh";
  } else if (language === "powershell") {
    return ".ps1";
  } else {
    throw new Error("Invalid language: " + language);
  }
}

export const exts = [
  ".fetch.ts",
  ".deno.ts",
  ".bun.ts",
  ".ts",
  ".py",
  ".go",
  ".sh",
  ".pg.sql",
  ".my.sql",
  ".bq.sql",
  ".sf.sql",
  ".ms.sql",
  ".sql",
  ".gql",
  ".ps1",
];

export function removeExtensionToPath(path: string): string {
  for (const ext of exts) {
    if (path.endsWith(ext)) {
      return path.substring(0, path.length - ext.length);
    }
  }
  throw new Error("Invalid extension: " + path);
}

async function list(opts: GlobalOptions & { showArchived?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Script[] = [];
  while (true) {
    const res = await ScriptService.listScripts({
      workspace: workspace.workspaceId,
      page,
      perPage,
      showArchived: opts.showArchived ?? false,
    });
    page += 1;
    total.push(...res);
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["path", "summary", "language", "created by"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.summary, x.language, x.created_by]))
    .render();
}

export async function resolve(input: string): Promise<Record<string, any>> {
  if (!input) {
    throw new Error("No data given");
  }

  if (input == "@-") {
    input = new TextDecoder().decode(await readAll(Deno.stdin));
  }
  if (input[0] == "@") {
    input = await Deno.readTextFile(input.substring(1));
  }
  try {
    return JSON.parse(input);
  } catch (e) {
    console.error("Impossible to parse input as JSON", input);
    throw e;
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  path: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};
  const id = await JobService.runScriptByPath({
    workspace: workspace.workspaceId,
    path,
    requestBody: input,
  });

  if (!opts.silent) {
    await track_job(workspace.workspaceId, id);
  }

  while (true) {
    try {
      const result =
        (
          await JobService.getCompletedJob({
            workspace: workspace.workspaceId,
            id,
          })
        ).result ?? {};

      if (opts.silent) {
        console.log(result);
      } else {
        log.info(result);
      }

      break;
    } catch {
      new Promise((resolve, _) => setTimeout(() => resolve(undefined), 100));
    }
  }
}

export async function track_job(workspace: string, id: string) {
  try {
    const result = await JobService.getCompletedJob({ workspace, id });

    log.info(result.logs);
    log.info("\n");
    log.info(colors.bold.underline.green("Job Completed"));
    log.info("\n");
    return;
  } catch {
    /* ignore */
  }

  log.info(colors.yellow("Waiting for Job " + id + " to start..."));

  let logOffset = 0;
  let running = false;
  let retry = 0;
  while (true) {
    let updates: {
      running?: boolean | undefined;
      completed?: boolean | undefined;
      new_logs?: string | undefined;
    };
    try {
      updates = await JobService.getJobUpdates({
        workspace,
        id,
        logOffset,
        running,
      });
    } catch {
      retry++;
      if (retry > 3) {
        log.info("failed to get job updated. skipping log streaming.");
        break;
      }
      continue;
    }

    if (!running && updates.running === true) {
      running = true;
      log.info(colors.green("Job running. Streaming logs..."));
    }

    if (updates.new_logs) {
      writeAllSync(Deno.stdout, new TextEncoder().encode(updates.new_logs));
      logOffset += updates.new_logs.length;
    }

    if (updates.completed === true) {
      running = false;
      break;
    }

    if (running && updates.running === false) {
      running = false;
      log.info(colors.yellow("Job suspended. Waiting for it to continue..."));
    }
  }
  await new Promise((resolve, _) => setTimeout(() => resolve(undefined), 1000));

  try {
    const final_job = await JobService.getCompletedJob({ workspace, id });
    if ((final_job.logs?.length ?? -1) > logOffset) {
      log.info(final_job.logs!.substring(logOffset));
    }
    log.info("\n");
    if (final_job.success) {
      log.info(colors.bold.underline.green("Job Completed"));
    } else {
      log.info(colors.bold.underline.red("Job Completed"));
    }
    log.info("\n");
  } catch {
    log.info("Job appears to have completed, but no data can be retrieved");
  }
}

async function show(opts: GlobalOptions, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const s = await ScriptService.getScriptByPath({
    workspace: workspace.workspaceId,
    path,
  });
  log.info(colors.underline(s.path));
  if (s.description) log.info(s.description);
  log.info("");
  log.info(s.content);
}

async function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  scriptPath: string,
  language: ScriptLanguage
) {
  if (!validatePath(scriptPath)) {
    return;
  }

  const scriptInitialCode = scriptBootstrapCode[language];
  if (scriptInitialCode === undefined) {
    throw new Error("Language unknown");
  }

  const config = await readConfigFile();

  const extension = filePathExtensionFromContentType(
    language,
    config.defaultTs
  );
  const scriptCodeFileFullPath = scriptPath + extension;
  const scriptMetadataFileFullPath = scriptPath + ".script.yaml";

  try {
    await Deno.stat(scriptCodeFileFullPath);
    await Deno.stat(scriptMetadataFileFullPath);
    throw new Error("File already exists in repository");
  } catch {
    // file does not exist, we can continue
  }

  const scriptMetadata = defaultScriptMetadata();
  if (opts.summary !== undefined) {
    scriptMetadata.summary = opts.summary;
  }
  if (opts.description !== undefined) {
    scriptMetadata.description = opts.description;
  }

  const scriptInitialMetadataYaml = yamlStringify(
    scriptMetadata as Record<string, any>,
    yamlOptions
  );

  Deno.writeTextFile(scriptCodeFileFullPath, scriptInitialCode, {
    createNew: true,
  });
  Deno.writeTextFile(scriptMetadataFileFullPath, scriptInitialMetadataYaml, {
    createNew: true,
  });
}

export type GlobalDeps = {
  pkgs: Record<string, string>;
  reqs: Record<string, string>;
};
export async function findGlobalDeps(): Promise<GlobalDeps> {
  const pkgs: { [key: string]: string } = {};
  const reqs: { [key: string]: string } = {};
  const els = await FSFSElement(Deno.cwd());
  for await (const entry of readDirRecursiveWithIgnore((p, isDir) => {
    p = "/" + p;
    return (
      !isDir && !(p.endsWith("/package.json") || p.endsWith("requirements.txt"))
    );
  }, els)) {
    if (entry.isDirectory || entry.ignored) continue;
    const content = await entry.getContentText();
    if (entry.path.endsWith("package.json")) {
      pkgs[entry.path.substring(0, entry.path.length - 12)] = content;
    } else if (entry.path.endsWith("requirements.txt")) {
      reqs[entry.path.substring(0, entry.path.length - 16)] = content;
    }
  }
  return { pkgs, reqs };
}
async function generateMetadata(
  opts: GlobalOptions & {
    lockOnly?: boolean;
    schemaOnly?: boolean;
    yes?: boolean;
  } & SyncOptions,
  scriptPath?: string
) {
  if (scriptPath == "") {
    scriptPath = undefined;
  }
  if (scriptPath && !validatePath(scriptPath)) {
    return;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);

  const globalDeps = await findGlobalDeps();
  if (scriptPath) {
    // read script metadata file
    await generateMetadataInternal(
      scriptPath,
      workspace,
      opts,
      false,
      false,
      globalDeps
    );
  } else {
    const ignore = await ignoreF(opts);
    const elems = await elementsToMap(
      await FSFSElement(Deno.cwd()),
      (p, isD) => {
        return (
          (!isD && !exts.some((ext) => p.endsWith(ext))) ||
          ignore(p, isD) ||
          p.includes(".flow/")
        );
      },
      false,
      {}
    );
    let hasAny = false;
    log.info("Generating metadata for all stale scripts:");
    for (const e of Object.keys(elems)) {
      const candidate = await generateMetadataInternal(
        e,
        workspace,
        opts,
        true,
        true,
        globalDeps
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.green(`+ ${candidate}`));
      }
    }
    if (hasAny) {
      if (
        !opts.yes &&
        !(await Confirm.prompt({
          message: "Update the metadata of the above scripts?",
          default: true,
        }))
      ) {
        return;
      }
    } else {
      log.info(colors.green.bold("No metadata to update"));
      return;
    }
    for (const e of Object.keys(elems)) {
      await generateMetadataInternal(
        e,
        workspace,
        opts,
        false,
        true,
        globalDeps
      );
    }
  }
}

const command = new Command()
  .description("script related commands")
  .option("--show-archived", "Enable archived scripts in output")
  .action(list as any)
  .command(
    "push",
    "push a local script spec. This overrides any remote versions. Use the script file (.ts, .js, .py, .sh)"
  )
  .arguments("<path:file>")
  .action(push as any)
  .command("show", "show a scripts content")
  .arguments("<path:file>")
  .action(show as any)
  .command("run", "run a script by path")
  .arguments("<path:file>")
  .option(
    "-d --data <data:file>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not output anything other then the final output. Useful for scripting."
  )
  .action(run as any)
  .command("bootstrap", "create a new script")
  .arguments("<path:file> <language:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any)
  .command(
    "generate-metadata",
    "re-generate the metadata file updating the lock and the script schema"
  )
  .arguments("[script:file]")
  .option("--yes", "Skip confirmation prompt")
  .option("--lock-only", "re-generate only the lock")
  .option("--schema-only", "re-generate only script schema")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account."
  )
  .action(generateMetadata as any);

export default command;
