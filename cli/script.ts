// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  JobService,
  readAll,
  Script,
  ScriptService,
  Table,
} from "./deps.ts";
import { Any, array, decoverto, model, property } from "./decoverto.ts";
import { writeAllSync } from "https://deno.land/std@0.176.0/streams/mod.ts";

@model()
export class ScriptFile {
  @property(() => String)
  parent_hash?: string;
  @property(() => String)
  summary: string;
  @property(() => String)
  description: string;
  @property(Any)
  schema?: any;
  @property(() => Boolean)
  is_template?: boolean;
  @property(array(() => String))
  lock?: Array<string>;
  @property({
    toInstance: (data) => {
      if (data == null) return data;

      if (
        data === "script" || data === "failure" || data === "trigger" ||
        data === "command" || data === "approvial"
      ) {
        return data;
      }

      throw new Error("Invalid kind " + data);
    },
    toPlain: (data) => data,
  })
  kind?: "script" | "failure" | "trigger" | "command" | "approval";

  constructor(summary: string, description: string) {
    this.summary = summary;
    this.description = description;
  }
}

type PushOptions = GlobalOptions;
async function push(
  opts: PushOptions,
  filePath: string,
  remotePath: string,
  contentPath?: string,
) {
  const workspace = await resolveWorkspace(opts);
  if (!await validatePath(opts, remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }
  if (!contentPath) {
    contentPath = await findContentFile(filePath);
  } else {
    const fstat = await Deno.stat(filePath);
    if (!fstat.isFile) {
      throw new Error("content path must refer to a file.");
    }
  }

  await requireLogin(opts);
  await pushScript(filePath, contentPath, workspace.workspaceId, remotePath);
  console.log(colors.bold.underline.green(`Script ${remotePath} pushed`));
}

export async function handleScriptMetadata(path: string, workspace: string, alreadySynced: string[]): Promise<boolean> {
  if (path.endsWith(".script.json")) {
    const contentPath = await findContentFile(path)
    return handleFile(contentPath, await Deno.readTextFile(contentPath), workspace, alreadySynced)
  } else {
    return false
  }
}

export async function handleFile(path: string, content: string, workspace: string, alreadySynced: string[]): Promise<boolean> {
  if (path.endsWith(".ts") || path.endsWith(".py") || path.endsWith(".go") || path.endsWith(".sh")) {
    if (alreadySynced.includes(path)) {
      return true
    }
    alreadySynced.push(path)
    const remotePath = path.substring(0, path.length - 3);
    const metaPath = remotePath + ".script.json";
    let typed = undefined
    try {
      await Deno.stat(metaPath)
      typed = JSON.parse(await Deno.readTextFile(metaPath))
      typed = decoverto.type(ScriptFile).plainToInstance(typed);
    } catch { }
    const language = inferContentTypeFromFilePath(path);

    try {
      const remote = await ScriptService.getScriptByPath({
        workspace,
        path: remotePath,
      });
      await ScriptService.createScript({
        workspace,
        requestBody: {
          content,
          description: typed.description,
          language,
          path: remotePath,
          summary: typed.summary,
          is_template: typed.is_template,
          kind: typed.kind,
          lock: typed.lock,
          parent_hash: remote.hash,
          schema: typed.schema,
        },
      });
      console.log(colors.yellow.bold(`Creating script with a parent ${remotePath}`))
    } catch {
      // no parent hash
      await ScriptService.createScript({
        workspace: workspace,
        requestBody: {
          content,
          description: typed.description,
          language,
          path: remotePath,
          summary: typed.summary,
          is_template: typed.is_template,
          kind: typed.kind,
          lock: typed.lock,
          parent_hash: undefined,
          schema: typed.schema,
        },
      });
      console.log(colors.yellow.bold(`Creating script without parent ${remotePath}`))

    }
    return true
  }
  return false
}

export async function findContentFile(filePath: string) {
  const candidates = [
    filePath.replace(".script.json", ".ts"),
    filePath.replace(".script.json", ".py"),
    filePath.replace(".script.json", ".go"),
    filePath.replace(".script.json", ".sh"),
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
      }),
    )
  )
    .filter((x) => x.file)
    .map((x) => x.path);
  if (validCandidates.length > 1) {
    throw new Error(
      "No content path given and more then one candidate found: " +
      validCandidates.join(", "),
    );
  }
  if (validCandidates.length < 1) {
    throw new Error("No content path given and no content file found.");
  }
  return validCandidates[0];
}

export function inferContentTypeFromFilePath(
  contentPath: string,
): "python3" | "deno" | "go" | "bash" {
  let language = contentPath.substring(contentPath.lastIndexOf("."));
  if (language == ".ts") language = "deno";
  if (language == ".py") language = "python3";
  if (language == ".sh") language = "bash";
  if (language == ".go") language = "go";
  if (
    language != "python3" && language != "deno" && language != "go" &&
    language != "bash"
  ) {
    throw new Error("Invalid language: " + language);
  }
  return language;
}

export async function pushScript(
  filePath: string,
  contentPath: string,
  workspace: string,
  remotePath: string,
) {
  const data = decoverto.type(ScriptFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  const content = await Deno.readTextFile(contentPath);

  const language = inferContentTypeFromFilePath(contentPath);
  let parent_hash = data.parent_hash;
  if (!parent_hash) {
    try {
      parent_hash = (
        await ScriptService.getScriptByPath({
          workspace: workspace,
          path: remotePath,
        })
      ).hash;
    } catch {
      /* no parent. New Script. */
    }
  }

  console.log(colors.bold.yellow("Pushing script..."));
  await ScriptService.createScript({
    workspace: workspace,
    requestBody: {
      path: remotePath,
      summary: data.summary,
      content: content,
      description: data.description,
      language: language,
      is_template: data.is_template,
      kind: data.kind,
      lock: data.lock,
      parent_hash: parent_hash,
      schema: data.schema,
    },
  });
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
    .body(
      total.map((x) => [
        x.path,
        x.summary,
        x.language,
        x.created_by,
      ]),
    )
    .render();
}

export async function resolve(input: string): Promise<Record<string, any>> {
  if (!input) {
    throw new Error("No data given");
  }

  if (input == "@-") {
    input = new TextDecoder().decode(await readAll(Deno.stdin));
  } if (input[0] == "@") {
    input = await Deno.readTextFile(input.substring(1));
  }
  try {
    return JSON.parse(input);
  } catch (e) {
    console.error("Impossible to parse input as JSON", input)
    throw e
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  path: string,
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
      const result = (
        await JobService.getCompletedJob({
          workspace: workspace.workspaceId,
          id,
        })
      ).result ?? {};
      console.log(result);

      break;
    } catch {
      new Promise((resolve, _) => setTimeout(() => resolve(undefined), 100));
    }
  }
}

export async function track_job(workspace: string, id: string) {
  try {
    const result = await JobService.getCompletedJob({ workspace, id });

    console.log(result.logs);
    console.log()
    console.log(colors.bold.underline.green("Job Completed"));
    console.log()
    return;
  } catch {
    /* ignore */
  }

  console.log(colors.yellow("Waiting for Job " + id + " to start..."));

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
        console.log("failed to get job updated. skipping log streaming.");
        break;
      }
      continue;
    }

    if (!running && updates.running === true) {
      running = true;
      console.log(colors.green("Job running. Streaming logs..."));
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
      console.log(
        colors.yellow("Job suspended. Waiting for it to continue..."),
      );
    }
  }
  await new Promise((resolve, _) => setTimeout(() => resolve(undefined), 1000));

  try {
    const final_job = await JobService.getCompletedJob({ workspace, id });
    if ((final_job.logs?.length ?? -1) > logOffset) {
      console.log(final_job.logs!.substring(logOffset));
    }
    console.log("\n")
    if (final_job.success) {
      console.log(colors.bold.underline.green("Job Completed"));

    } else {
      console.log(colors.bold.underline.red("Job Completed"));
    }
    console.log()

  } catch {
    console.log("Job appears to have completed, but no data can be retrieved");
  }
}

async function show(opts: GlobalOptions, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const s = await ScriptService.getScriptByPath({
    workspace: workspace.workspaceId,
    path,
  });
  console.log(colors.underline(s.path));
  if (s.description) console.log(s.description);
  console.log("");
  console.log(s.content);
}

const command = new Command()
  .description("script related commands")
  .option("--show-archived", "Enable archived scripts in output")
  .action(list as any)
  .command(
    "push",
    "push a local script spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <remote_path:string> [content_path:string]")
  .action(push as any)
  .command("show", "show a scripts content")
  .arguments("<path:string>")
  .action(show as any)
  .command("run", "run a script by path")
  .arguments("<path:string>")
  .option(
    "-d --data <data:string>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-.",
  )
  .option(
    "-s --silent",
    "Do not ouput anything other then the final output. Useful for scripting.",
  )
  .action(run as any);

export default command;
