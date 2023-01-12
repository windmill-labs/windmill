// deno-lint-ignore-file no-explicit-any
import { colors, Command, path } from "./deps.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { findContentFile, pushScript } from "./script.ts";
import { ScriptFile } from "./ScriptFile";
import { GlobalOptions, inferTypeFromPath } from "./types.ts";
import { ResourceTypeFile } from "./resource-type.ts";
import { FolderFile } from "./folder.ts";
import { decoverto } from "./decoverto.ts";

type Candidate = {
  path: string;
  namespaceKind: "user" | "group" | "folder";
  namespaceName: string;
};

type ResourceTypeCandidate = {
  path: string;
};

type FolderCandidate = {
  path: string;
  namespaceName: string;
};

async function findCandidateFiles(
  dir: string,
): Promise<
  {
    normal: Candidate[];
    resourceTypes: ResourceTypeCandidate[];
    folders: FolderCandidate[];
  }
> {
  dir = path.resolve(dir);
  if (path.dirname(dir).startsWith(".")) {
    return { normal: [], resourceTypes: [], folders: [] };
  }
  const normalCandidates: Candidate[] = [];
  const resourceTypeCandidates: ResourceTypeCandidate[] = [];
  const folderCandidates: FolderCandidate[] = [];
  for await (const e of Deno.readDir(dir)) {
    if (e.isDirectory) {
      if (e.name == "u" || e.name == "g" || e.name == "f") { // TODO: Check version for f
        const newDir = dir + (dir.endsWith("/") ? "" : "/") + e.name;
        for await (const e2 of Deno.readDir(newDir)) {
          if (e2.isDirectory) {
            if (e2.name.startsWith(".")) continue;
            const namespaceName = e2.name;
            const stack: string[] = [];
            {
              const path = newDir + "/" + namespaceName + "/";
              stack.push(path);
              try {
                await Deno.stat(path + "folder.meta.json");
                folderCandidates.push({
                  namespaceName,
                  path: path + "folder.meta.json",
                });
              } catch {}
            }

            while (stack.length > 0) {
              const dir2 = stack.pop()!;
              for await (const e3 of Deno.readDir(dir2)) {
                if (e3.isFile) {
                  if (e3.name === "folder.meta.json") continue;
                  normalCandidates.push({
                    path: dir2 + e3.name,
                    namespaceKind: e.name == "g"
                      ? "group"
                      : e.name == "u"
                      ? "user"
                      : "folder",
                    namespaceName: namespaceName,
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
            "Including organizational folder " + e.name + " in push!",
          ),
        );
        const { normal, resourceTypes, folders } = await findCandidateFiles(
          path.join(dir, e.name),
        );
        normalCandidates.push(...normal);
        resourceTypeCandidates.push(...resourceTypes);
        folderCandidates.push(...folders);
      }
    } else {
      // handle root files
      if (e.name.endsWith(".resource-type.json")) {
        resourceTypeCandidates.push({
          path: dir + (dir.endsWith("/") ? "" : "/") + e.name,
        });
      }
    }
  }
  return {
    normal: normalCandidates,
    folders: folderCandidates,
    resourceTypes: resourceTypeCandidates,
  };
}

async function push(opts: GlobalOptions, dir?: string) {
  dir = dir ?? Deno.cwd();
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log(colors.blue("Searching Directory..."));
  const { normal, resourceTypes, folders } = await findCandidateFiles(dir);
  console.log(
    colors.blue(
      "Found " + (normal.length + resourceTypes.length + folders.length) +
        " candidates",
    ),
  );
  for (const resourceType of resourceTypes) {
    const fileName = resourceType.path.substring(
      resourceType.path.lastIndexOf("/") + 1,
    );
    const fileNameParts = fileName.split(".");
    // invalid file names, like my.cool.script.script.json. Not valid.
    if (fileNameParts.length != 3) {
      console.log(
        colors.yellow("invalid file name found at " + resourceType.path),
      );
      continue;
    }

    // filter out non-json files. Note that we filter out script contents above, so this is really an error.
    if (fileNameParts.at(-1) != "json") {
      console.log(colors.yellow("non-JSON file found at " + resourceType.path));
      continue;
    }

    console.log("pushing resource type " + fileNameParts.at(-3)!);
    await decoverto.type(ResourceTypeFile).rawToInstance(
      await Deno.readTextFile(resourceType.path),
    ).push(workspace.workspaceId, fileNameParts.at(-3)!);
  }
  for (const folder of folders) {
    await decoverto.type(FolderFile).plainToInstance(
      JSON.parse(await Deno.readTextFile(folder.path)),
    ).push(
      workspace.workspaceId,
      "f/" + folder.namespaceName,
    );
  }
  for (const candidate of normal) {
    // full file name. No leading /. includes .type.json
    const fileName = candidate.path.substring(
      candidate.path.lastIndexOf("/") + 1,
    );
    // figure out just the path after ...../u|g/username|group/ (in extra dir)
    const dirParts = candidate.path.split("/").filter((x) => x.length > 0);
    // TODO: check version for folder
    const gIndex = dirParts.findIndex((x) => x == "u" || x == "g" || x == "f");
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
        colors.yellow("invalid file name found at " + candidate.path),
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
      console.log(
        colors.yellow(
          "Found resource type file at " +
            candidate.path +
            " this appears to be inside a path folder. Resource types are not addressed by path. Place them at the root or inside only an organizational folder. Ignoring this file!",
        ),
      );
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
          "file with invalid type " + type + " found at " + candidate.path,
        ),
      );
      continue;
    }

    // create the remotePath for the API
    const remotePath = (candidate.namespaceKind === "group"
      ? "g/"
      : (candidate.namespaceKind === "user" ? "u/" : "f/")) +
      candidate.namespaceName +
      "/" +
      (extraDir.length > 0 ? extraDir + "/" : "") +
      fileNameParts.at(-3);

    console.log("pushing " + type + " to " + remotePath);

    const typed = inferTypeFromPath(
      candidate.path,
      JSON.parse(await Deno.readTextFile(candidate.path)),
    );
    if (typed instanceof ResourceTypeFile || typed instanceof FolderFile) {
      throw new Error(
        "Resource Types and Folders should  be filtered out at this point!",
      );
    } else if (typed instanceof ScriptFile) {
      let contentPath: string;
      try {
        contentPath = await findContentFile(candidate.path);
      } catch (e) {
        console.log(colors.red(e.toString()));
        continue;
      }
      await pushScript(
        candidate.path,
        contentPath,
        workspace.workspaceId,
        remotePath,
      );
    } else {
      typed.push(workspace.workspaceId, remotePath);
    }
  }
  console.log(colors.underline.bold.green("Successfully Pushed all files."));
}

const command = new Command()
  .description("Push all files from a folder")
  .arguments("[dir:string]")
  .action(push as any);

export default command;
