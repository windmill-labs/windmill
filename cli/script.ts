import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { ScriptService } from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getContext } from "./context.ts";
import {
  Job,
  JobService,
  Script,
} from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";
import { green } from "https://deno.land/std@0.161.0/fmt/colors.ts";

type ScriptFile = {
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
};

type PushOptions = GlobalOptions;
async function push(
  opts: PushOptions,
  filePath: string,
  remotePath: string,
  contentPath?: string
) {
  const { workspace } = await getContext(opts);
  if (!(remotePath.startsWith("g") || remotePath.startsWith("u"))) {
    console.log(
      colors.red(
        "Given remote path looks invalid. Remote paths are typicall of the form <u|g>/<username|group>/..."
      )
    );
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

  await pushScript(filePath, contentPath, workspace, remotePath);
  console.log(colors.bold.underline.green("Script successfully pushed"));
}

export async function findContentFile(filePath: string) {
  const candidates = [
    filePath.replace(".json", ".ts"),
    filePath.replace(".json", ".py"),
    filePath.replace(".json", ".go"),
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
      "No content path given and more then one candidate found: " +
        validCandidates.join(", ")
    );
  }
  if (validCandidates.length < 1) {
    throw new Error("No content path given and no content file found.");
  }
  return validCandidates[0];
}

export async function pushScript(
  filePath: string,
  contentPath: string,
  workspace: string,
  remotePath: string
) {
  const data: ScriptFile = JSON.parse(await Deno.readTextFile(filePath));
  const content = await Deno.readTextFile(contentPath);

  let language = contentPath.substring(contentPath.lastIndexOf("."));
  if (language == ".ts") language = "deno";
  if (language == ".py") language = "python3";
  if (language == ".go") language = "go";
  if (language != "python3" && language != "deno" && language != "go") {
    throw new Error("Invalid language: " + language);
  }

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
  const { workspace } = await getContext(opts);

  let page = 0;
  const perPage = 10;
  const total: Script[] = [];
  while (true) {
    const res = await ScriptService.listScripts({
      workspace,
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
    .header(["path", "hash", "kind", "language", "created at", "created by"])
    .padding(2)
    .border(true)
    .body(
      total.map((x) => [
        x.path,
        x.hash,
        x.kind,
        x.language,
        x.created_at,
        x.created_by,
      ])
    )
    .render();
}

async function run(
  opts: GlobalOptions & {
    input: Record<string, any>;
  },
  path: string
) {
  const { workspace } = await getContext(opts);
  let id = await JobService.runScriptByPath({
    workspace,
    path,
    requestBody: opts.input,
  });

  track_job(workspace, id);
}

export async function track_job(workspace: string, id: string) {
  try {
    const result = await JobService.getCompletedJob({ workspace, id });

    console.log(result.logs);
    console.log(colors.bold.underline.green("Job Completed"));
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
      console.log(updates.new_logs);
      logOffset += updates.new_logs.length;
    }

    if (updates.completed === true) {
      console.log("completed");
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

    if (final_job.success) {
      console.log(colors.bold.underline.green("Job Completed"));
    } else {
      console.log(colors.bold.underline.red("Job Completed"));
    }
  } catch {
    console.log("Job appears to have completed, but no data can be retrieved");
  }
}

async function show(opts: GlobalOptions, path: string) {
  const { workspace } = await getContext(opts);
  const s = await ScriptService.getScriptByPath({ workspace, path });
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
    "push a local script spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string> [content_path:string]")
  .action(push as any)
  .command("show", "show a scripts content")
  .arguments("<path:string>")
  .action(show as any)
  .command("run", "run a script by path")
  .arguments("<path:string>")
  .option("--input.* <input>", "Inputs to pass to the script")
  .action(run as any);

export default command;
