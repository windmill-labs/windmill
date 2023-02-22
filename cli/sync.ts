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
  FlowService
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
import {
  handleScriptMetadata,
  ScriptFile,
} from "./script.ts";
import { ResourceFile } from "./resource.ts";
import { FlowFile } from "./flow.ts";
import { VariableFile } from "./variable.ts";
import { handleFile } from "./script.ts";
import { equal } from "https://deno.land/x/equal/mod.ts";

type DynFSElement = {
  isDirectory: boolean;
  path: string;
  getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<DynFSElement>;
};

async function FSFSElement(p: string): Promise<DynFSElement> {
  function _internal_element(
    localP: string,
    isDir: boolean,
  ): DynFSElement {
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
  ignore: (path: string) => boolean,
  root: DynFSElement,
): AsyncGenerator<
  {
    path: string;
    ignored: boolean;
    isDirectory: boolean;
    getContentBytes(): Promise<Uint8Array>;
    getContentText(): Promise<string>;
  }
> {
  const stack: {
    path: string;
    isDirectory: boolean;
    ignored: boolean;
    c(): AsyncIterable<DynFSElement>;
    getContentBytes(): Promise<Uint8Array>;
    getContentText(): Promise<string>;
  }[] = [{
    path: root.path,
    ignored: ignore(root.path),
    isDirectory: root.isDirectory,
    c: root.getChildren,
    getContentBytes(): Promise<Uint8Array> {
      throw undefined;
    },
    getContentText(): Promise<string> {
      throw undefined;
    },
  }];

  while (stack.length > 0) {
    const e = stack.pop()!;
    yield e;
    if (!e.isDirectory) continue;
    for await (const e2 of e.c()) {
      stack.push({
        path: e2.path,
        ignored: e.ignored || ignore(e2.path),
        isDirectory: e2.isDirectory,
        getContentBytes: e2.getContentBytes,
        getContentText: e2.getContentText,
        c: e2.getChildren,
      });
    }
  }
}

type Added = { name: "added"; path: string; content: string };
type Deleted = { name: "deleted"; path: string; };
type Edit = { name: "edited"; path: string; before: string; after: string; };

type Change = Added | Deleted | Edit;

async function elementsToMap(els: DynFSElement, ignore: (path: string) => boolean): Promise<{ [key: string]: string }> {
  const map: { [key: string]: string } = {};
  for await (const entry of readDirRecursiveWithIgnore(
    ignore,
    els,
  )) {
    if (entry.isDirectory || entry.ignored) continue;
    const content = await entry.getContentText();
    map[entry.path] = content;
  }
  return map;
}
async function compareDynFSElement(
  els1: DynFSElement, els2: DynFSElement,
  ignore: (path: string) => boolean,
  raw: boolean
): Promise<Change[]> {

  const [m1, m2] = raw ? [await elementsToMap(els1, ignore), {}] :
    await Promise.all([elementsToMap(els1, ignore), elementsToMap(els2, ignore)]);

  const changes: Change[] = [];

  for (const [k, v] of Object.entries(m1)) {
    if (m2[k] === undefined) {
      changes.push({ name: "added", path: k, content: v });
    } else if (m2[k] != v && (!k.endsWith(".json") || !equal(JSON.parse(v), JSON.parse(m2[k])))) {
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

  return changes
}


async function ignoreF() {
  try {
    const ignore: {
      accepts(file: string): boolean;
      denies(file: string): boolean;
    } = gitignore_parser.compile(
      await Deno.readTextFile(".wmillignore"),
    );
    return (p: string) => p.startsWith(".wmill") || ignore.denies(p);
  } catch (e) {
    return (p: string) => p.startsWith(".wmill");
  }
}

async function pull(
  opts: GlobalOptions & { raw: boolean; yes: boolean },
) {


  if (!opts.raw) {
    await ensureDir(path.join(Deno.cwd(), ".wmill"));
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);


  console.log("Computing diff local vs remote ...");
  const remote = ZipFSElement((await downloadZip(workspace))!)
  const local = await FSFSElement(path.join(Deno.cwd(), opts.raw ? "" : ".wmill"))
  const changes = await compareDynFSElement(remote, local, await ignoreF(), opts.raw)


  console.log(`remote -> local: ${changes.length} changes to apply`);
  if (changes.length > 0) {

    prettyChanges(changes)
    if (
      !opts.yes && !(await Confirm.prompt({ message: `Do you want to apply these ${changes.length} changes?`, default: true }))
    ) {
      return
    }
    console.log(`Applying changes to files ...`);
    for await (const change of changes) {
      const target = path.join(Deno.cwd(), change.path);
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path)
      if (change.name === "edited") {

        try {
          if (await Deno.readTextFile(target) !== change.before && !opts.yes) {
            console.log(colors.red(`Conflict detected on ${change.path}\nBoth local and remote have been modified.`))
            if (await Confirm.prompt("Preserve local (push to change remote and avoid seeing this again)?")) {
              continue;
            }
          }
        } catch { }
        if (change.path.endsWith(".json")) {
          const diffs =
            microdiff(
              JSON.parse(change.before),
              JSON.parse(change.after),
              { cyclesFix: false },
            )

          console.log(`Editing ${getTypeStrFromPath(change.path)} json ${change.path}`)
          await applyDiff(
            diffs,
            target,
          );
        } else {
          console.log(`Editing script ${change.path}`)
          await Deno.writeTextFile(target, change.after);
        }
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget))
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "added") {
        await ensureDir(path.dirname(target))
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget))
          console.log(`Adding ${getTypeStrFromPath(change.path)} ${change.path}`)
        }
        await Deno.writeTextFile(target, change.content);
        if (!opts.raw) {
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "deleted") {
        try {
          console.log(`Deleting ${getTypeStrFromPath(change.path)} ${change.path}`)
          await Deno.remove(target)
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
    console.log(colors.green.underline(`Done! All ${changes.length} changes pushed to the remote.`));
  }


  async function applyDiff(diffs: Difference[], file: string) {
    ensureDir(path.dirname(file));
    let json;
    try {
      json = JSON.parse(await Deno.readTextFile(file));
    } catch {
      json = {};
    }
    // TODO: Delegate the below to the object itself
    // This would work by infering the type of `JSON` (which includes then statically typing it using decoverto) and then
    // delegating the applying of the diffs to the object via an interface
    for (const diff of diffs) {
      if (diff.type === "CREATE") {
        setValueByPath(json, diff.path, diff.value);
      } else if (diff.type === "REMOVE") {
        setValueByPath(json, diff.path, undefined);
      } else if (diff.type === "CHANGE") {
        setValueByPath(json, diff.path, diff.value);
      }
    }

    await Deno.writeTextFile(file, JSON.stringify(json, undefined, "  "), {
      create: true,
    });
  }
}




function prettyChanges(changes: Change[]) {
  for (const change of changes) {
    if (change.name === "added") {
      console.log(colors.green(`+ ${getTypeStrFromPath(change.path)} ` + change.path));
    } else if (change.name === "deleted") {
      console.log(colors.red(`- ${getTypeStrFromPath(change.path)} ` + change.path));
    } else if (change.name === "edited") {
      console.log(colors.yellow(`~ ${getTypeStrFromPath(change.path)} ` + change.path));
    }
  }
}

function prettyDiff(diffs: Difference[]) {
  for (const diff of diffs) {
    let pathString = "";
    for (const pathSegment of diff.path) {
      if (typeof pathSegment === "string") {
        pathString += ".";
        pathString += pathSegment;
      } else {
        pathString += "[";
        pathString += pathSegment;
        pathString += "]";
      }
    }
    if (diff.type === "REMOVE" || diff.type === "CHANGE") {
      console.log(colors.red("- " + pathString + " = " + diff.oldValue));
    }
    if (diff.type === "CREATE" || diff.type === "CHANGE") {
      console.log(colors.green("+ " + pathString + " = " + diff.value));
    }
  }
}


function removeSuffix(str: string, suffix: string) {
  return str.slice(0, str.length - suffix.length);
}

async function push(opts: GlobalOptions & { raw: boolean, yes: boolean, skipPull: boolean }) {


  if (!opts.raw) {
    if (!opts.skipPull) {
      console.log("You need to be up-to-date before pushing, pulling first.")
      await pull(opts)
    }
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);


  console.log("Computing diff remote vs local ...");
  const remote = ZipFSElement((await downloadZip(workspace))!)
  const local = await FSFSElement(path.join(Deno.cwd(), ""))
  const changes = await compareDynFSElement(local, remote, await ignoreF(), opts.raw)

  console.log(`local -> remote: ${changes.length} changes to apply`);
  if (changes.length > 0) {

    prettyChanges(changes)
    if (
      !opts.yes && !(await Confirm.prompt({ message: `Do you want to apply these ${changes.length} changes?`, default: true }))
    ) {
      return
    }
    console.log(`Applying changes to files ...`);
    const alreadySynced: string[] = []
    for await (const change of changes) {
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path)
      if (change.name === "edited") {
        if (await handleScriptMetadata(change.path, workspace.workspaceId, alreadySynced)) {
          continue
        } else if (await handleFile(change.path, change.after, workspace.workspaceId, alreadySynced)) {
          continue
        }
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget))
          console.log(`Editing ${getTypeStrFromPath(change.path)} ${change.path}`)
        }
        const obj = inferTypeFromPath(change.path, JSON.parse(change.after))

        const diff = microdiff(inferTypeFromPath(change.path, JSON.parse(change.before)), obj, { cyclesFix: false });
        await applyDiff(
          workspace.workspaceId,
          change.path.split(".")[0],
          obj,
          diff,
        );
        if (!opts.raw) {
          await Deno.writeTextFile(stateTarget, change.after);
        }
      } else if (change.name === "added") {
        if (change.path.endsWith(".script.json")) {
          continue
        } else if (await handleFile(change.path, change.content, workspace.workspaceId, alreadySynced)) {
          continue
        }
        if (!opts.raw) {
          await ensureDir(path.dirname(stateTarget))
          console.log(`Adding ${getTypeStrFromPath(change.path)} ${change.path}`)
        }
        const obj = inferTypeFromPath(change.path, JSON.parse(change.content))
        const diff = microdiff({}, obj, { cyclesFix: false });
        await applyDiff(
          workspace.workspaceId,
          change.path.split(".")[0],
          obj,
          diff,
        );
        if (!opts.raw) {
          await Deno.writeTextFile(stateTarget, change.content);
        }
      } else if (change.name === "deleted") {
        if (!change.path.includes(".json")) {
          continue
        }
        console.log(`Deleting ${getTypeStrFromPath(change.path)} ${change.path}`)
        const typ = getTypeStrFromPath(change.path)
        const workspaceId = workspace.workspaceId;
        switch (typ) {
          case "script": {
            const script = await ScriptService.getScriptByPath({ workspace: workspaceId, path: removeSuffix(change.path, ".script.json") })
            await ScriptService.deleteScriptByHash({ workspace: workspaceId, hash: script.hash })
            break;
          }
          case "folder":
            await FolderService.deleteFolder({ workspace: workspaceId, name: change.path.split('/')[1] })
            break;
          case "resource":
            await ResourceService.deleteResource({ workspace: workspaceId, path: removeSuffix(change.path, ".resource.json") })
            break;
          case "resource-type":
            await ResourceService.deleteResourceType({ workspace: workspaceId, path: removeSuffix(change.path, ".resource-type.json") })
            break
          case "flow":
            await FlowService.archiveFlowByPath({ workspace: workspaceId, path: removeSuffix(change.path, ".flow.json") })
            break
          case "app":
            await AppService.deleteApp({ workspace: workspaceId, path: removeSuffix(change.path, ".app.json") })
            break
          case "variable":
            await VariableService.deleteVariable({ workspace: workspaceId, path: removeSuffix(change.path, ".variable.json") })
            break
          default:
            break;
        }
      }
    }
    console.log(colors.green.underline(`Done! All ${changes.length} changes applied locally.`));

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
  ) {
    if (file instanceof ScriptFile) {
      throw new Error(
        "This code path should be unreachable - we should never generate diffs for scripts",
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
      console.log("No diffs to apply to " + remotePath)
      return;
    }
    try {
      await file.pushDiffs(workspace, remotePath, diffs);
    } catch (e) {
      console.error("Failing to apply diffs to " + remotePath)
      console.error(e.body)
    }
  }
}

const command = new Command()
  .command("pull")
  .description(
    "Pull any remote changes and apply them locally. Use --raw for usage without local state tracking.",
  )
  .option("--yes", "Pull without needing confirmation")
  .option("--raw", "Pull without using state, just overwrite.")
  .action(pull as any)
  .command("push")
  .description(
    "Push any local changes and apply them remotely. Use --raw for usage without local state tracking.",
  )
  .option("--skip-pull", "Push without pulling first")
  .option("--yes", "Push without needing confirmation")
  .option("--raw", "Push without using state, just overwrite.")
  .action(push as any);

export default command;
