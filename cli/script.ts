// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  JobService,
  log,
  NewScript,
  readAll,
  Script,
  ScriptService,
  Table,
  writeAllSync,
  yamlParse,
} from "./deps.ts";

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
  await handleFile(filePath, workspace.workspaceId, []);
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function handleScriptMetadata(
  path: string,
  workspace: string,
  alreadySynced: string[]
): Promise<boolean> {
  if (path.endsWith(".script.json") || path.endsWith(".script.yaml")) {
    const contentPath = await findContentFile(path);
    return handleFile(contentPath, workspace, alreadySynced);
  } else {
    return false;
  }
}

export async function handleFile(
  path: string,
  workspace: string,
  alreadySynced: string[]
): Promise<boolean> {
  if (
    !path.includes(".inline_script.") &&
    (path.endsWith(".ts") ||
      path.endsWith(".py") ||
      path.endsWith(".go") ||
      path.endsWith(".sh") ||
      path.endsWith(".sql") ||
      path.endsWith(".gql") ||
      path.endsWith(".ps1"))
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    log.debug(`Processing local script ${path}`);

    alreadySynced.push(path);
    const remotePath = path
      .substring(0, path.indexOf("."))
      .replaceAll("\\", "/");
    const metaPath = remotePath + ".script.json";
    let typed = undefined;
    try {
      await Deno.stat(metaPath);
      typed = JSON.parse(await Deno.readTextFile(metaPath));
    } catch {
      const metaPath = remotePath + ".script.yaml";
      try {
        await Deno.stat(metaPath);
        typed = yamlParse(await Deno.readTextFile(metaPath));
      } catch {
        // no meta file
      }
    }

    const language = inferContentTypeFromFilePath(path);

    let remote = undefined;
    try {
      remote = await ScriptService.getScriptByPath({
        workspace,
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
            typed.is_template === remote.is_template &&
            typed.kind == remote.kind &&
            !remote.archived &&
            remote?.lock == typed.lock?.join("\n") &&
            JSON.stringify(typed.schema) == JSON.stringify(remote.schema))
        ) {
          log.info(
            colors.yellow(
              `No change to push for script ${remotePath}, skipping`
            )
          );
          return true;
        }
      }
      log.info(
        colors.yellow.bold(`Creating script with a parent ${remotePath}`)
      );
      await ScriptService.createScript({
        workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
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
        },
      });
    } else {
      log.info(
        colors.yellow.bold(`Creating script without parent ${remotePath}`)
      );
      // no parent hash
      await ScriptService.createScript({
        workspace: workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
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
        },
      });
    }
    return true;
  }
  return false;
}

export async function findContentFile(filePath: string) {
  const candidates = filePath.endsWith("script.json")
    ? [
        filePath.replace(".script.json", ".fetch.ts"),
        filePath.replace(".script.json", ".bun.ts"),
        filePath.replace(".script.json", ".ts"),
        filePath.replace(".script.json", ".py"),
        filePath.replace(".script.json", ".go"),
        filePath.replace(".script.json", ".sh"),
        filePath.replace(".script.json", "pg.sql"),
        filePath.replace(".script.json", "my.sql"),
        filePath.replace(".script.json", "bq.sql"),
        filePath.replace(".script.json", "sf.sql"),
        filePath.replace(".script.json", ".gql"),
        filePath.replace(".script.json", ".ps1"),
      ]
    : [
        filePath.replace(".script.yaml", ".fetch.ts"),
        filePath.replace(".script.yaml", ".bun.ts"),
        filePath.replace(".script.yaml", ".ts"),
        filePath.replace(".script.yaml", ".py"),
        filePath.replace(".script.yaml", ".go"),
        filePath.replace(".script.yaml", ".sh"),
        filePath.replace(".script.yaml", "pg.sql"),
        filePath.replace(".script.yaml", "bq.sql"),
        filePath.replace(".script.yaml", "sf.sql"),
        filePath.replace(".script.yaml", ".gql"),
        filePath.replace(".script.yaml", ".ps1"),
      ];
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
    throw new Error("No content path given and no content file found.");
  }
  return validCandidates[0];
}

export function inferContentTypeFromFilePath(
  contentPath: string
):
  | "python3"
  | "deno"
  | "bun"
  | "nativets"
  | "go"
  | "bash"
  | "powershell"
  | "postgresql"
  | "mysql"
  | "bigquery"
  | "snowflake"
  | "mssql"
  | "graphql" {
  if (contentPath.endsWith(".py")) {
    return "python3";
  } else if (contentPath.endsWith("fetch.ts")) {
    return "nativets";
  } else if (contentPath.endsWith("bun.ts")) {
    return "bun";
  } else if (contentPath.endsWith(".ts")) {
    return "deno";
  } else if (contentPath.endsWith(".go")) {
    return "go";
  } else if (contentPath.endsWith(".my.sql")) {
    return "mysql";
  } else if (contentPath.endsWith(".bq.sql")) {
    return "bigquery";
  } else if (contentPath.endsWith(".sf.sql")) {
    return "snowflake";
  } else if (contentPath.endsWith(".ms.sql")) {
    return "mssql";
  } else if (contentPath.endsWith(".pg.sql")) {
    return "postgresql";
  } else if (contentPath.endsWith(".gql")) {
    return "graphql";
  } else if (contentPath.endsWith(".sh")) {
    return "bash";
  } else if (contentPath.endsWith(".ps1")) {
    return "powershell";
  } else {
    throw new Error(
      "Invalid language: " + contentPath.substring(contentPath.lastIndexOf("."))
    );
  }
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
      log.info(result);

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
    "Do not ouput anything other then the final output. Useful for scripting."
  )
  .action(run as any);

export default command;
