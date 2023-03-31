import { requireLogin, resolveWorkspace } from "./context.ts";
import {
  colors,
  Command,
  Confirm,
  ensureDir,
  gitignore_parser,
  JSZip,
  microdiff,
  path,
  ScriptService,
  FolderService,
  ResourceService,
  VariableService,
  AppService,
  FlowService,
} from "./deps.ts";
import {
  Difference,
  getTypeStrFromPath,
  GlobalOptions,
  inferTypeFromPath,
  setValueByPath,
} from "./types.ts";
import { downloadZip } from "./pull.ts";
import { FolderFile } from "./folder.ts";
import { ResourceTypeFile } from "./resource-type.ts";
import { handleScriptMetadata, ScriptFile } from "./script.ts";
import { ResourceFile } from "./resource.ts";
import { FlowFile } from "./flow.ts";
import { VariableFile } from "./variable.ts";
import { handleFile } from "./script.ts";
import { equal } from "https://deno.land/x/equal@v1.5.0/mod.ts";
import * as Diff from "npm:diff";

type DynFSElement = {
  isDirectory: boolean;
  path: string;
  getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<DynFSElement>;
};

async function FSFSElement(p: string): Promise<DynFSElement> {
  function _internal_element(localP: string, isDir: boolean): DynFSElement {
    return {
      isDirectory: isDir,
      path: localP.substring(p.length + 1),
      async *getChildren(): AsyncIterable<DynFSElement> {
        for await (const e of Deno.readDir(localP)) {
          yield _internal_element(path.join(localP, e.name), e.isDirectory);
        }
      },
      async getContentBytes(): Promise<Uint8Array> {
        return await Deno.readFile(localP);
      },
      async getContentText(): Promise<string> {
        return await Deno.readTextFile(localP);
      },
    };
  }
  return _internal_element(p, (await Deno.stat(p)).isDirectory);
}

function ZipFSElement(zip: JSZip): DynFSElement {
  function _internal_file(p: string, f: JSZip.JSZipObject): DynFSElement {
    return {
      isDirectory: false,
      path: p,
      // deno-lint-ignore require-yield
      async *getChildren(): AsyncIterable<DynFSElement> {
        throw new Error("Cannot get children of file");
      },
      async getContentBytes(): Promise<Uint8Array> {
        return await f.async("uint8array");
      },
      async getContentText(): Promise<string> {
        return await f.async("text");
      },
    };
  }
  function _internal_folder(p: string, zip: JSZip): DynFSElement {
    return {
      isDirectory: true,
      path: p,
      async *getChildren(): AsyncIterable<DynFSElement> {
        for (const filename in zip.files) {
          const file = zip.files[filename];
          const totalPath = path.join(p, filename);
          if (file.dir) {
            const e = zip.folder(file.name)!;
            yield _internal_folder(totalPath, e);
          } else {
            yield _internal_file(totalPath, file);
          }
        }
      },
      async getContentBytes(): Promise<Uint8Array> {
        throw new Error("Cannot get content of folder");
      },
      async getContentText(): Promise<string> {
        throw new Error("Cannot get content of folder");
      },
    };
  }
  return _internal_folder("./", zip);
}

async function* readDirRecursiveWithIgnore(
  ignore: (path: string, isDirectory: boolean) => boolean,
  root: DynFSElement
): AsyncGenerator<{
  path: string;
  ignored: boolean;
  isDirectory: boolean;
  getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
}> {
  const stack: {
    path: string;
    isDirectory: boolean;
    ignored: boolean;
    c(): AsyncIterable<DynFSElement>;
    getContentBytes(): Promise<Uint8Array>;
    getContentText(): Promise<string>;
  }[] = [
    {
      path: root.path,
      ignored: ignore(root.path, root.isDirectory),
      isDirectory: root.isDirectory,
      c: root.getChildren,
      getContentBytes(): Promise<Uint8Array> {
        throw undefined;
      },
      getContentText(): Promise<string> {
        throw undefined;
      },
    },
  ];

  while (stack.length > 0) {
    const e = stack.pop()!;
    yield e;
    if (!e.isDirectory) continue;
    for await (const e2 of e.c()) {
      stack.push({
        path: e2.path,
        ignored: e.ignored || ignore(e2.path, e2.isDirectory),
        isDirectory: e2.isDirectory,
        getContentBytes: e2.getContentBytes,
        getContentText: e2.getContentText,
        c: e2.getChildren,
      });
    }
  }
}

type Added = { name: "added"; path: string; content: string };
type Deleted = { name: "deleted"; path: string };
type Edit = { name: "edited"; path: string; before: string; after: string };

type Change = Added | Deleted | Edit;

async function elementsToMap(
  els: DynFSElement,
  ignore: (path: string, isDirectory: boolean) => boolean
): Promise<{ [key: string]: string }> {
  const map: { [key: string]: string } = {};
  for await (const entry of readDirRecursiveWithIgnore(ignore, els)) {
    if (entry.isDirectory || entry.ignored) continue;
    const content = await entry.getContentText();
    map[entry.path] = content;
  }
  return map;
}
async function compareDynFSElement(
  els1: DynFSElement,
  els2: DynFSElement | undefined,
  ignore: (path: string, isDirectory: boolean) => boolean
): Promise<Change[]> {
  const [m1, m2] = els2
    ? await Promise.all([
        elementsToMap(els1, ignore),
        elementsToMap(els2, ignore),
      ])
    : [await elementsToMap(els1, ignore), {}];

  const changes: Change[] = [];

  for (const [k, v] of Object.entries(m1)) {
    if (m2[k] === undefined) {
      changes.push({ name: "added", path: k, content: v });
    } else if (
      m2[k] != v &&
      (!k.endsWith(".json") || !equal(JSON.parse(v), JSON.parse(m2[k])))
    ) {
      // await Deno.writeTextFile("/tmp/k", m2[k])
      // await Deno.writeTextFile("/tmp/v", v)
      // console.log(k)
      // if (k.includes("flow"))
      //   Deno.exit(1)
      changes.push({ name: "edited", path: k, after: v, before: m2[k] });
    }
  }

  for (const [k] of Object.entries(m2)) {
    if (m1[k] === undefined) {
      changes.push({ name: "deleted", path: k });
    }
  }

  return changes;
}

const isNotWmillFile = (p: string, isDirectory: boolean) => {
  if (p.endsWith("/")) {
    return false;
  }
  if (isDirectory) {
    return !p.startsWith("u/") && !p.startsWith("f/") && !p.startsWith("g/");
  }

  try {
    const typ = getTypeStrFromPath(p);
    if (typ == "resource-type") {
      return p.includes("/");
    } else {
      return !p.startsWith("u/") && !p.startsWith("f/") && !p.startsWith("g/");
    }
  } catch {
    return true;
  }
};

const isWhitelisted = (p: string) => {
  return p == "./" || p == "" || p == "u" || p == "f" || p == "g";
};
async function ignoreF() {
  try {
    const ignore: {
      accepts(file: string): boolean;
      denies(file: string): boolean;
    } = gitignore_parser.compile(await Deno.readTextFile(".wmillignore"));

    return (p: string, isDirectory: boolean) => {
      return (
        !isWhitelisted(p) &&
        (isNotWmillFile(p, isDirectory) || ignore.denies(p))
      );
    };
  } catch (e) {
    return (p: string, isDirectory: boolean) =>
      !isWhitelisted(p) && isNotWmillFile(p, isDirectory);
  }
}

async function pull(
  opts: GlobalOptions & {
    raw: boolean;
    yes: boolean;
    failConflicts: boolean;
    plainSecrets?: boolean;
  }
) {
  if (!opts.raw) {
    await ensureDir(path.join(Deno.cwd(), ".wmill"));
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log(
    colors.gray(
      "Computing the files to update locally to match remote (taking .wmillignore into account)"
    )
  );
  const remote = ZipFSElement(
    (await downloadZip(workspace, opts.plainSecrets))!
  );
  const local = opts.raw
    ? undefined
    : await FSFSElement(path.join(Deno.cwd(), opts.raw ? "" : ".wmill"));
  const changes = await compareDynFSElement(remote, local, await ignoreF());

  console.log(`remote -> local: ${changes.length} changes to apply`);
  if (changes.length > 0) {
    prettyChanges(changes);
    if (
      !opts.yes &&
      !opts.raw &&
      !(await Confirm.prompt({
        message: `Do you want to apply these ${changes.length} changes?`,
        default: true,
      }))
    ) {
      return;
    }

    const conflicts = [];
    console.log(colors.gray(`Applying changes to files ...`));
    for await (const change of changes) {
      const target = path.join(Deno.cwd(), change.path);
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path);
      if (change.name === "edited") {
        try {
          const currentLocal = await Deno.readTextFile(target);
          if (currentLocal !== change.before && currentLocal !== change.after) {
            console.log(
              colors.red(
                `Conflict detected on ${change.path}\nBoth local and remote have been modified.`
              )
            );
            if (opts.failConflicts) {
              conflicts.push({
                local: currentLocal,
                change,
                path: change.path,
              });
              continue;
            } else if (opts.yes) {
              console.log(
                colors.red(
                  `Override local version with remote since --yes was passed and no --fail-conflicts.`
                )
              );
            } else {
              showConflict(change.path, currentLocal, change.after);
              if (
                await Confirm.prompt(
                  "Preserve local (push to change remote and avoid seeing this again)?"
                )
              ) {
                continue;
              }
            }
          }
        } catch {}
        if (!change.path.endsWith(".json")) {
          console.log(`Editing script ${change.path}`);
        }
        await Deno.writeTextFile(target, change.after);

        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget));
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "added") {
        await ensureDir(path.dirname(target));
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget));
          console.log(
            `Adding ${getTypeStrFromPath(change.path)} ${change.path}`
          );
        }
        await Deno.writeTextFile(target, change.content);
        if (!opts.raw) {
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "deleted") {
        try {
          console.log(
            `Deleting ${getTypeStrFromPath(change.path)} ${change.path}`
          );
          await Deno.remove(target);
          if (!opts.raw) {
            await Deno.remove(stateTarget);
          }
        } catch (e) {
          if (!opts.raw) {
            await Deno.remove(stateTarget);
          }
        }
      }
    }
    if (opts.failConflicts) {
      if (conflicts.length > 0) {
        console.error(colors.red(`Conflicts were found`));
        console.log("Conflicts:");
        for (const conflict of conflicts) {
          showConflict(conflict.path, conflict.local, conflict.change.after);
        }
        console.log(
          colors.red(`Please resolve theses conflicts manually by either:
  - reverting the content back to its remote (\`wmill pull\` and refuse to preserve local when prompted)
  - pushing the changes with \`wmill push --skip-pull\` to override wmill with all your local changes
`)
        );
        Deno.exit(1);
      }
    }
    console.log(
      colors.green.underline(
        `Done! All ${changes.length} changes applied locally.`
      )
    );
  }

  function showConflict(path: string, local: string, remote: string) {
    console.log(colors.yellow(`- ${path}`));

    let finalString = "";
    for (const part of Diff.diffLines(local, remote)) {
      if (part.removed) {
        // print red if removed without newline
        finalString += `\x1b[31m${part.value}\x1b[0m`;
      } else if (part.added) {
        // print green if added
        finalString += `\x1b[32m${part.value}\x1b[0m`;
      } else {
        // print white if unchanged
        finalString += `\x1b[37m${part.value}\x1b[0m`;
      }
    }
    console.log(finalString);
    console.log("\x1b[31mlocal\x1b[31m - \x1b[32mremote\x1b[32m");
    console.log();
  }
}

function prettyChanges(changes: Change[]) {
  for (const change of changes) {
    if (change.name === "added") {
      console.log(
        colors.green(`+ ${getTypeStrFromPath(change.path)} ` + change.path)
      );
    } else if (change.name === "deleted") {
      console.log(
        colors.red(`- ${getTypeStrFromPath(change.path)} ` + change.path)
      );
    } else if (change.name === "edited") {
      console.log(
        colors.yellow(`~ ${getTypeStrFromPath(change.path)} ` + change.path)
      );
    }
  }
}

// function prettyDiff(diffs: Difference[]) {
//   for (const diff of diffs) {
//     let pathString = "";
//     for (const pathSegment of diff.path) {
//       if (typeof pathSegment === "string") {
//         pathString += ".";
//         pathString += pathSegment;
//       } else {
//         pathString += "[";
//         pathString += pathSegment;
//         pathString += "]";
//       }
//     }
//     if (diff.type === "REMOVE" || diff.type === "CHANGE") {
//       console.log(colors.red("- " + pathString + " = " + diff.oldValue));
//     }
//     if (diff.type === "CREATE" || diff.type === "CHANGE") {
//       console.log(colors.green("+ " + pathString + " = " + diff.value));
//     }
//   }
// }

function removeSuffix(str: string, suffix: string) {
  return str.slice(0, str.length - suffix.length);
}

async function push(
  opts: GlobalOptions & {
    raw: boolean;
    yes: boolean;
    skipPull: boolean;
    failConflicts: boolean;
    plainSecrets?: boolean;
  }
) {
  if (!opts.raw) {
    if (!opts.skipPull) {
      console.log(
        colors.gray("You need to be up-to-date before pushing, pulling first.")
      );
      await pull(opts);
      console.log(colors.green("Pull done, now pushing."));
      console.log();
    }
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log(
    colors.gray(
      "Computing the files to update on the remote to match local (taking .wmillignore into account)"
    )
  );
  const remote = opts.raw
    ? undefined
    : ZipFSElement((await downloadZip(workspace, opts.plainSecrets))!);
  const local = await FSFSElement(path.join(Deno.cwd(), ""));
  const changes = await compareDynFSElement(local, remote, await ignoreF());

  console.log(`remote <- local: ${changes.length} changes to apply`);
  if (changes.length > 0) {
    prettyChanges(changes);
    if (
      !opts.yes &&
      !(await Confirm.prompt({
        message: `Do you want to apply these ${changes.length} changes?`,
        default: true,
      }))
    ) {
      return;
    }
    console.log(colors.gray(`Applying changes to files ...`));
    const alreadySynced: string[] = [];
    for await (const change of changes) {
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path);
      let stateExists = true;
      try {
        await Deno.stat(stateTarget);
      } catch {
        stateExists = false;
      }

      if (change.name === "edited") {
        if (
          await handleScriptMetadata(
            change.path,
            workspace.workspaceId,
            alreadySynced
          )
        ) {
          if (!opts.raw && stateExists) {
            await Deno.writeTextFile(stateTarget, change.after);
          }
          continue;
        } else if (
          await handleFile(
            change.path,
            change.after,
            workspace.workspaceId,
            alreadySynced
          )
        ) {
          if (!opts.raw && stateExists) {
            await Deno.writeTextFile(stateTarget, change.after);
          }
          continue;
        }
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget));
          console.log(
            `Editing ${getTypeStrFromPath(change.path)} ${change.path}`
          );
        }
        const obj = inferTypeFromPath(change.path, JSON.parse(change.after));

        const diff = microdiff(
          inferTypeFromPath(change.path, JSON.parse(change.before)),
          obj,
          { cyclesFix: false }
        );
        await applyDiff(
          workspace.workspaceId,
          change.path.split(".")[0],
          obj,
          diff,
          opts.plainSecrets
        );
        if (!opts.raw && stateExists) {
          await Deno.writeTextFile(stateTarget, change.after);
        }
      } else if (change.name === "added") {
        if (change.path.endsWith(".script.json")) {
          continue;
        } else if (
          await handleFile(
            change.path,
            change.content,
            workspace.workspaceId,
            alreadySynced
          )
        ) {
          continue;
        }
        if (!opts.raw && stateExists) {
          await ensureDir(path.dirname(stateTarget));
          console.log(
            `Adding ${getTypeStrFromPath(change.path)} ${change.path}`
          );
        }
        const obj = inferTypeFromPath(change.path, JSON.parse(change.content));
        const diff = microdiff({}, obj, { cyclesFix: false });
        await applyDiff(
          workspace.workspaceId,
          change.path.split(".")[0],
          obj,
          diff,
          opts.plainSecrets
        );
        if (!opts.raw && stateExists) {
          await Deno.writeTextFile(stateTarget, change.content);
        }
      } else if (change.name === "deleted") {
        if (!change.path.includes(".json")) {
          continue;
        }
        console.log(
          `Deleting ${getTypeStrFromPath(change.path)} ${change.path}`
        );
        const typ = getTypeStrFromPath(change.path);
        const workspaceId = workspace.workspaceId;
        switch (typ) {
          case "script": {
            const script = await ScriptService.getScriptByPath({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".script.json"),
            });
            await ScriptService.deleteScriptByHash({
              workspace: workspaceId,
              hash: script.hash,
            });
            break;
          }
          case "folder":
            await FolderService.deleteFolder({
              workspace: workspaceId,
              name: change.path.split("/")[1],
            });
            break;
          case "resource":
            await ResourceService.deleteResource({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".resource.json"),
            });
            break;
          case "resource-type":
            await ResourceService.deleteResourceType({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".resource-type.json"),
            });
            break;
          case "flow":
            await FlowService.deleteFlowByPath({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".flow.json"),
            });
            break;
          case "app":
            await AppService.deleteApp({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".app.json"),
            });
            break;
          case "variable":
            await VariableService.deleteVariable({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".variable.json"),
            });
            break;
          default:
            break;
        }
        try {
          await Deno.remove(stateTarget);
        } catch {}
      }
    }
    console.log(
      colors.green.underline(
        `Done! All ${changes.length} changes pushed to the remote workspace.`
      )
    );
  }

  async function applyDiff(
    workspace: string,
    remotePath: string,
    file:
      | ScriptFile
      | VariableFile
      | FlowFile
      | ResourceFile
      | ResourceTypeFile
      | FolderFile,
    diffs: Difference[],
    plainSecrets?: boolean
  ) {
    if (file instanceof ScriptFile) {
      throw new Error(
        "This code path should be unreachable - we should never generate diffs for scripts"
      );
    } else if (file instanceof FolderFile) {
      const parts = remotePath.split("/");
      if (parts[0] === "f") {
        remotePath = parts[1];
      } else {
        remotePath = parts[0];
      }
    }
    if (diffs.length === 0) {
      console.log("No diffs to apply to " + remotePath);
      return;
    }
    try {
      await file.pushDiffs(workspace, remotePath, diffs, plainSecrets);
    } catch (e) {
      console.error("Failing to apply diffs to " + remotePath);
      console.error(JSON.stringify(e));
    }
  }
}

const command = new Command()
  .command("pull")
  .description(
    "Pull any remote changes and apply them locally. Use --raw for usage without local state tracking."
  )
  .option(
    "--fail-conflicts",
    "Error on conflicts (both remote and local have changes on the same item)"
  )
  .option("--yes", "Pull without needing confirmation")
  .option("--raw", "Pull without using state, just overwrite.")
  .option("--plain-secrets", "Pull secrets as plain text")
  .action(pull as any)
  .command("push")
  .description(
    "Push any local changes and apply them remotely. Use --raw for usage without local state tracking."
  )
  .option(
    "--fail-conflicts",
    "Error on conflicts (both remote and local have changes on the same item)"
  )
  .option("--skip-pull", "Push without pulling first (you have pulled prior)")
  .option("--yes", "Push without needing confirmation")
  .option("--raw", "Push without using state, just overwrite.")
  .option("--plain-secrets", "Push secrets as plain text")
  .action(push as any);

export default command;
