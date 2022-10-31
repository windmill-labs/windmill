import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  ScriptService,
  setClient,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { getToken } from "./login.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import * as fs from "https://deno.land/std@0.161.0/fs/mod.ts";

type Options = GlobalOptions;
type ScriptFile = {
  workspace_id: string;
  path: string;
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
};

async function push(
  { baseUrl }: Options,
  filePath: string,
  contentPath?: string
) {
  setClient(await getToken(baseUrl), baseUrl);

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }
  if (!contentPath) {
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
    contentPath = validCandidates[0];
  } else {
    const fstat = await Deno.stat(filePath);
    if (!fstat.isFile) {
      throw new Error("content path must refer to a file.");
    }
  }

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
          workspace: data.workspace_id,
          path: data.path,
        })
      ).hash;
    } catch {
      /* no parent. New Script. */
    }
  }

  console.log(colors.bold.yellow("Pushing script..."));
  await ScriptService.createScript({
    workspace: data.workspace_id,
    requestBody: {
      path: data.path,
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
  console.log(colors.bold.underline.green("Script successfully pushed"));
}

const command = new Command()
  .description("script related commands")
  .command(
    "push",
    "push a local script spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> [content_path:string]")
  .action(push as any);

export default command;
