import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import * as path from "https://deno.land/std@0.162.0/path/mod.ts";
import { getContext } from "./context.ts";
import { pushFlow } from "./flow.ts";
import { pushResource } from "./resource.ts";
import { findContentFile, pushScript } from "./script.ts";
import { GlobalOptions } from "./types.ts";
import { pushVariable } from "./variable.ts";
import { pushResourceType } from "./resource-type.ts";

type Candidate = {
  path: string;
  group: boolean;
  groupOrUsername: string;
};
async function findCandidateFiles(dir: string): Promise<Candidate[]> {
  const candidates: Candidate[] = [];
  for await (const e of Deno.readDir(dir)) {
    if (e.isDirectory) {
      if (e.name == "u" || e.name == "g") {
        const newDir = dir + (dir.endsWith("/") ? "" : "/") + e.name;
        for await (const e2 of Deno.readDir(newDir)) {
          if (e2.isDirectory) {
            const groupOrUserName = e2.name;
            const stack: string[] = [];
            stack.push(newDir + "/" + groupOrUserName + "/");

            while (stack.length > 0) {
              const dir2 = stack.pop()!;
              for await (const e3 of Deno.readDir(dir2)) {
                if (e3.isFile) {
                  candidates.push({
                    path: dir2 + e3.name,
                    group: e.name == "g",
                    groupOrUsername: groupOrUserName,
                  });
                } else {
                  stack.push(dir2 + e3.name + "/");
                }
              }
            }
          }
        }
      } else {
        console.log(
          colors.yellow(
            "Including organizational folder " + e.name + " in push!"
          )
        );
        candidates.push(...(await findCandidateFiles(path.join(dir, e.name))));
      }
    } else {
      // handle root files
      if (e.name.endsWith(".resource-type.json")) {
        candidates.push({
          group: false,
          groupOrUsername: "",
          path: dir + (dir.endsWith("/") ? "" : "/") + e.name,
        });
        console.log(candidates);
      }
    }
  }
  return candidates;
}

async function push(opts: GlobalOptions, dir?: string) {
  dir = dir ?? Deno.cwd();
  const { workspace } = await getContext(opts);

  console.log(colors.blue("Searching Directory..."));
  const candidates: Candidate[] = await findCandidateFiles(dir);
  console.log(colors.blue("Found " + candidates.length + " candidates"));
  for (const candidate of candidates) {
    // full file name. No leading /. includes .type.json
    const fileName = candidate.path.substring(
      candidate.path.lastIndexOf("/") + 1
    );
    // figure out just the path after ...../u|g/username|group/ (in extra dir)
    const dirParts = candidate.path.split("/").filter((x) => x.length > 0);
    const gIndex = dirParts.findIndex((x) => x == "u" || x == "g");
    const extraDir = dirParts.slice(gIndex + 2, -1).join("/");

    // file name parts has .json (hopefully) at -1, type at -2, and the actual name at -3. Dots in names are not allowed.
    const fileNameParts = fileName.split(".");

    // filter out script content files
    if (
      fileNameParts.at(-1) == "ts" ||
      fileNameParts.at(-1) == "py" ||
      fileNameParts.at(-1) == "go"
    ) {
      // probably part of a script. Silent ignore.
      continue;
    }

    // invalid file names, like my.cool.script.script.json. Not valid.
    if (fileNameParts.length != 3) {
      console.log(
        colors.yellow("invalid file name found at " + candidate.path)
      );
      continue;
    }

    // filter out non-json files. Note that we filter out script contents above, so this is really an error.
    if (fileNameParts.at(-1) != "json") {
      console.log(colors.yellow("non-JSON file found at " + candidate.path));
      continue;
    }

    // get the type & filter it for valid ones.
    const type = fileNameParts.at(-2);
    if (type == "resource-type") {
      if (candidate.group == false && candidate.groupOrUsername == "") {
        console.log("pushing resource type " + fileNameParts.at(-3)!);
        await pushResourceType(
          workspace,
          candidate.path,
          fileNameParts.at(-3)!
        );
      } else {
        console.log(
          colors.yellow(
            "Found resource type file at " +
              candidate.path +
              " this appears to be inside a path folder. Resource types are not addressed by path. Place them at the root or inside only an organizational folder. Ignoring this file!"
          )
        );
      }
      continue;
    }

    if (
      type != "flow" &&
      type != "resource" &&
      type != "script" &&
      type != "variable"
    ) {
      console.log(
        colors.yellow(
          "file with invalid type " + type + " found at " + candidate.path
        )
      );
      continue;
    }

    // create the remotePath for the API
    const remotePath =
      (candidate.group ? "g/" : "u/") +
      candidate.groupOrUsername +
      "/" +
      (extraDir.length > 0 ? extraDir + "/" : "") +
      fileNameParts.at(-3);

    console.log("pushing " + type + " to " + remotePath);

    if (type == "flow") {
      await pushFlow(candidate.path, workspace, remotePath);
    } else if (type == "resource") {
      await pushResource(workspace, candidate.path, remotePath);
    } else if (type == "script") {
      let contentPath: string;
      try {
        contentPath = await findContentFile(candidate.path);
      } catch (e) {
        console.log(colors.red(e));
        continue;
      }
      await pushScript(candidate.path, contentPath, workspace, remotePath);
    } else if (type == "variable") {
      await pushVariable(workspace, candidate.path, remotePath);
    }
  }
  console.log(colors.underline.bold.green("Successfully Pushed all files."));
}

const command = new Command()
  .description("Push all files from a folder")
  .arguments("[dir:string]")
  .action(push as any);

export default command;
