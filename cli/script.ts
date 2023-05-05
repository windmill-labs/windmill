// deno-lint-ignore-file no-explicit-any
import { GlobalOptions, parseFromFile } from "./types.ts";
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
import { writeAllSync } from "https://deno.land/std@0.176.0/streams/mod.ts";
import { parse as yamlParse } from "https://deno.land/std@0.184.0/yaml/mod.ts";

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
  const remotePath = filePath.split(".")[0];

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }
  let contentPath: string;
  let metaPath: string | undefined;
  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    metaPath = filePath;
    contentPath = await findContentFile(filePath);
  } else {
    contentPath = filePath;
    metaPath = undefined;
  }

  await requireLogin(opts);
  await pushScript(metaPath, contentPath, workspace.workspaceId, remotePath);
  console.log(colors.bold.underline.green(`Script ${remotePath} pushed`));
}

export async function handleScriptMetadata(
  path: string,
  workspace: string,
  alreadySynced: string[]
): Promise<boolean> {
  if (path.endsWith(".script.json") || path.endsWith(".script.yaml")) {
    const contentPath = await findContentFile(path);
    return handleFile(
      contentPath,
      await Deno.readTextFile(contentPath),
      workspace,
      alreadySynced
    );
  } else {
    return false;
  }
}

export async function handleFile(
  path: string,
  content: string,
  workspace: string,
  alreadySynced: string[]
): Promise<boolean> {
  if (
    path.endsWith(".ts") ||
    path.endsWith(".py") ||
    path.endsWith(".go") ||
    path.endsWith(".sh")
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    alreadySynced.push(path);
    const remotePath = path.substring(0, path.length - 3);
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
        path: remotePath,
      });
    } catch {
      // no remote script
    }

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
          console.log(
            colors.yellow(
              `No change to push for script ${remotePath}, skipping`
            )
          );
          return true;
        }
      }
      await ScriptService.createScript({
        workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language,
          path: remotePath,
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
          kind: typed?.kind,
          lock: typed?.lock,
          parent_hash: remote.hash,
          schema: typed?.schema,
        },
      });

      console.log(
        colors.yellow.bold(`Creating script with a parent ${remotePath}`)
      );
    } else {
      // no parent hash
      await ScriptService.createScript({
        workspace: workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language,
          path: remotePath,
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
          kind: typed?.kind,
          lock: typed?.lock,
          parent_hash: undefined,
          schema: typed?.schema,
        },
      });
      console.log(
        colors.yellow.bold(`Creating script without parent ${remotePath}`)
      );
    }
    return true;
  }
  return false;
}

export async function findContentFile(filePath: string) {
  const candidates = filePath.endsWith("script.json")
    ? [
        filePath.replace(".script.json", ".ts"),
        filePath.replace(".script.json", ".py"),
        filePath.replace(".script.json", ".go"),
        filePath.replace(".script.json", ".sh"),
      ]
    : [
        filePath.replace(".script.yaml", ".ts"),
        filePath.replace(".script.yaml", ".py"),
        filePath.replace(".script.yaml", ".go"),
        filePath.replace(".script.yaml", ".sh"),
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
): "python3" | "deno" | "go" | "bash" {
  let language = contentPath.substring(contentPath.lastIndexOf("."));
  if (language == ".ts") language = "deno";
  if (language == ".py") language = "python3";
  if (language == ".sh") language = "bash";
  if (language == ".go") language = "go";
  if (
    language != "python3" &&
    language != "deno" &&
    language != "go" &&
    language != "bash"
  ) {
    throw new Error("Invalid language: " + language);
  }
  return language;
}

export async function pushScript(
  filePath: string | undefined,
  contentPath: string,
  workspace: string,
  remotePath: string
) {
  const data: ScriptFile | undefined = filePath
    ? parseFromFile(filePath)
    : undefined;
  const content = await Deno.readTextFile(contentPath);

  const language = inferContentTypeFromFilePath(contentPath);
  let parent_hash = data?.parent_hash;
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
      summary: data?.summary ?? "",
      content: content,
      description: data?.description ?? "",
      language: language,
      is_template: data?.is_template,
      kind: data?.kind,
      lock: data?.lock,
      parent_hash: parent_hash,
      schema: data?.schema,
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
    console.log();
    console.log(colors.bold.underline.green("Job Completed"));
    console.log();
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
        colors.yellow("Job suspended. Waiting for it to continue...")
      );
    }
  }
  await new Promise((resolve, _) => setTimeout(() => resolve(undefined), 1000));

  try {
    const final_job = await JobService.getCompletedJob({ workspace, id });
    if ((final_job.logs?.length ?? -1) > logOffset) {
      console.log(final_job.logs!.substring(logOffset));
    }
    console.log("\n");
    if (final_job.success) {
      console.log(colors.bold.underline.green("Job Completed"));
    } else {
      console.log(colors.bold.underline.red("Job Completed"));
    }
    console.log();
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
    "push a local script spec. This overrides any remote versions. Can use a script file (.ts, .js, .py, .sh) or a script spec file (.json). "
  )
  .arguments("<file_path:file>")
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
