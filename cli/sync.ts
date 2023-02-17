import { requireLogin, resolveWorkspace } from "./context.ts";
import { getWorkspaceStream, Workspace } from "./workspace.ts";
import { decoverto, map, MapShape, model, property } from "./decoverto.ts";
import {
  cbor,
  colors,
  Command,
  Confirm,
  ensureDir,
  gitignore_parser,
  JSZip,
  microdiff,
  nanoid,
  objectHash,
  path,
  ScriptService,
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
  findContentFile,
  inferContentTypeFromFilePath,
  pushScript,
  ScriptFile,
} from "./script.ts";
import { ResourceFile } from "./resource.ts";
import { FlowFile } from "./flow.ts";
import { VariableFile } from "./variable.ts";

type TrackedId = string;
const TrackedId = String;

const CONTENT_ENCODER: cbor.Encoder = new cbor.Encoder({ pack: true });

export class Tracked {
  #parent: State;

  path: string;

  #content_cache?: string;

  constructor(parent: State, path: string) {
    this.#parent = parent;
    this.path = path;
  }

  async getContent(): Promise<unknown | undefined> {
    if (this.#content_cache) {
      return this.#content_cache;
    }

    const f = this.#parent.stateContentFor(this.path);
    if (!f) {
      return undefined;
    }

    const data = await Deno.readFile(
      State.getInternalWmillFolder(f),
    );
    const content = CONTENT_ENCODER.decode(data);
    this.#content_cache = content;
    return content;
  }

  getHash(): string | undefined {
    return this.#parent.hashes.get(objectHash(this.path));
  }
}

type DynFSElement = {
  isDirectory: boolean;
  path: string;
  getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<DynFSElement>;
};

async function FSFSElement(p: string): Promise<DynFSElement> {
  function _internal_element(
    p: string,
    isDir: boolean,
  ): DynFSElement {
    return {
      isDirectory: isDir,
      path: p,
      async *getChildren(): AsyncIterable<DynFSElement> {
        for await (const e of Deno.readDir(p)) {
          yield _internal_element(path.join(p, e.name), e.isDirectory);
        }
      },
      async getContentBytes(): Promise<Uint8Array> {
        return await Deno.readFile(p);
      },
      async getContentText(): Promise<string> {
        return await Deno.readTextFile(p);
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

@model()
export class State {
  // TODO: Handle script contents, as they are separate files & need to be tracked separately.
  // I think the way this makes most sense is actually reating them as a separate kind of file everywhere
  // And then move to using a diff-based push system everywhere. This should greatly simplify code & enable this easily

  @property(map(() => TrackedId, () => String, { shape: MapShape.Object }))
  hashes: Map<TrackedId, string>;

  @property(map(() => TrackedId, () => String, { shape: MapShape.Object }))
  contentFiles: Map<TrackedId, string>;

  @property(() => String)
  workspaceId: string;

  @property(() => String)
  remoteUrl: string;

  constructor(
    hashes: Map<TrackedId, string>,
    contentFiles: Map<TrackedId, string>,
    workspaceId: string,
    remoteUrl: string,
  ) {
    this.hashes = hashes;
    this.contentFiles = contentFiles;
    this.workspaceId = workspaceId;
    this.remoteUrl = remoteUrl;
  }

  public stateContentFor(path: string) {
    const hashOfPath = objectHash(path);
    const file = this.contentFiles.get(hashOfPath);
    return file;
  }

  public get(path: string): Tracked {
    // TODO: Normalize path
    return new Tracked(this, path);
  }

  public static getInternalWmillFolder(...subpath: string[]): string {
    return path.join(Deno.cwd(), ".wmill", ...subpath);
  }

  public async *getFiles(): AsyncGenerator<{
    localFile: string | undefined;
    stateFile: Tracked;
    isIgnored: boolean;
    path: string;
  }> {
    // not sure why the auto-typing doesn't work here, see <https://www.npmjs.com/package/gitignore-parser> (a @types package is available...)
    const ignore: {
      accepts(file: string): boolean;
      denies(file: string): boolean;
    } = gitignore_parser.compile(
      await Deno.readTextFile(".wmillignore"),
    );
    const base = Deno.cwd();
    for await (
      const { ignored, path, getContentText, isDirectory }
        of readDirRecursiveWithIgnore(
          ignore.denies,
          await FSFSElement(base),
        )
    ) {
      if (isDirectory || path.includes(".wmill")) {
        continue;
      }
      const path2 = path.substring(base.length + 1);
      const stateFile = this.get(path2);
      let localFile: string | undefined;
      try {
        localFile = await getContentText();
      } catch {
        localFile = undefined;
      }
      yield { path: path2, isIgnored: ignored, stateFile, localFile };
    }
  }

  public async save(): Promise<void> {
    const encoder = new cbor.Encoder({});
    const plain = decoverto.type(State).instanceToPlain(this);
    const result: Uint8Array = encoder.encode(plain, {});
    await Deno.writeFile(State.getInternalWmillFolder("main"), result, {
      create: true,
    });
  }

  public static async loadState(): Promise<State> {
    const source = await Deno.readFile(this.getInternalWmillFolder("main"));
    const encoder = new cbor.Encoder({});
    const raw = encoder.decode(source);
    const state = decoverto.type(State).plainToInstance(
      raw,
    );
    return Object.freeze(state);
  }
}

async function getState(opts: GlobalOptions) {
  const existingState = await State.loadState();

  const workspaceStream = await getWorkspaceStream();
  const reader = workspaceStream.getReader();
  while (true) {
    const res = await reader.read();
    if (res.value) {
      if (
        new URL(res.value.remote) == new URL(existingState.remoteUrl) &&
        res.value.workspaceId == existingState.workspaceId
      ) {
        opts.workspace = res.value.name;
        (opts as any).__secret_workspace = res.value;
        break;
      }
    }
    if (res.done) {
      break;
    }
  }
  return existingState;
}

async function updateStateFromRemote(
  workspace: Workspace,
  state: State,
  callback: (filename: string) => PromiseLike<boolean> | boolean,
) {
  const zipDir = await downloadZip(workspace);
  if (!zipDir) throw new Error("Failed to pull Zip");

  for await (
    const entry of readDirRecursiveWithIgnore(
      (_) => false,
      ZipFSElement(zipDir),
    )
  ) {
    if (entry.isDirectory || entry.ignored) continue;
    const e = state.get(entry.path);
    if (e) {
      const val = await entry.getContentText();
      if (entry.path.endsWith(".json")) {
        const parsed = JSON.parse(val);
        const typed = inferTypeFromPath(entry.path, parsed);

        const oldHash = e.getHash();
        const newHash = objectHash(typed);

        if (!oldHash || oldHash !== newHash) {
          if (!await callback(entry.path)) {
            return; // notice that we are not saving
          }

          try {
            await Deno.stat(entry.path);
          } catch {
            await ensureDir(path.dirname(e.path));
            await Deno.writeTextFile(e.path, "{}");
          }
          state.hashes.set(objectHash(e.path), newHash);

          const encoded = CONTENT_ENCODER.encode(typed);

          const fileName = nanoid();
          await Deno.writeFile(
            State.getInternalWmillFolder(fileName),
            encoded,
            { create: true },
          );
          state.contentFiles.set(objectHash(e.path), fileName);
        }
      } else {
        const fileName = nanoid();
        await Deno.writeTextFile(
          State.getInternalWmillFolder(fileName),
          val,
          { create: true },
        );
        state.contentFiles.set(objectHash(e.path), fileName);
        try {
          await Deno.stat(entry.path);
        } catch {
          await ensureDir(path.dirname(e.path));
          await Deno.writeTextFile(e.path, "tmp_file");
        }
      }
    }
  }

  await state.save();
}

async function pull(
  opts: GlobalOptions & { raw: boolean; rawOverride: boolean },
) {
  if (opts.raw) {
    const opts2 = opts as any;
    opts2.override = opts2.rawOverride;
    opts2.raw = undefined;
    opts2.rawOverride = undefined;
    await pullRaw(opts2);
    return;
  }

  const state = await getState(opts);
  console.log(state);

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log("Pulling remote changes");
  await updateStateFromRemote(workspace, state, (_) => true);

  const diffs = diffState(state);

  await copyNonJsonFiles(state);

  console.log(`Applying changes to files`);
  for await (const diff of diffs) {
    await applyDiff(
      diff.diff,
      path.join(Deno.cwd(), diff.localPath),
    );
  }
  console.log(colors.green.underline("Done! All changes applied."));
  console.log(state);

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

    // TODO: Use decoverto instanceToPlain below.
    await Deno.writeTextFile(file, JSON.stringify(json, undefined, "  "), {
      create: true,
    });
  }

  async function copyNonJsonFiles(state: State): Promise<void> {
    for await (
      const { path, localFile, stateFile, isIgnored } of state.getFiles()
    ) {
      if (path.endsWith(".json") || isIgnored || !localFile) {
        continue;
      }

      const target = await Deno.open(stateFile.path, {
        create: true,
        write: true,
      });
      const source = await Deno.open(
        State.getInternalWmillFolder(
          state.stateContentFor(stateFile.path)!,
        ),
        {
          read: true,
          write: false,
        },
      );
      await source.readable.pipeTo(target.writable);
    }
  }
}

async function* diffState(state: State): AsyncGenerator<StateDiff, void, void> {
  for await (
    const { path, localFile, stateFile, isIgnored } of state.getFiles()
  ) {
    if (!path.endsWith(".json") || isIgnored) {
      continue;
    }
    const fileText = localFile ?? "{}";
    const old = JSON.parse(fileText);
    const fileHash = objectHash(old);

    if (fileHash !== stateFile.getHash()) {
      const stateContent = await stateFile.getContent() as any;

      const diff = microdiff(
        old,
        stateContent ?? {},
        { cyclesFix: false },
      );

      yield new StateDiff(stateFile.path, diff);
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

async function push(opts: GlobalOptions & { raw: boolean }) {
  if (opts.raw) {
    const opts2 = opts as any;
    opts2.raw = undefined;
    await pushRaw(opts2, undefined);
    return;
  }

  const state = await getState(opts);

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let error = false;
  await updateStateFromRemote(workspace, state, async (filename) => {
    const e = state.get(filename);
    if (!e) {
      throw new Error("!? State change on untracked file ?!");
    }

    let fileJSON;
    try {
      fileJSON = JSON.parse(
        await Deno.readTextFile(path.join(Deno.cwd(), e.path)),
      );
    } catch {
      fileJSON = {};
    }
    const file = inferTypeFromPath(e.path, fileJSON);
    const eContent = (inferTypeFromPath(e.path, await e.getContent())) ?? {};

    const fileHash = objectHash(file);
    const eHash = objectHash(eContent);

    if (fileHash !== eHash) {
      console.log(
        colors.red("!! Local and Remote change present. Local diff:"),
      );
      prettyDiff(microdiff(eContent as any, file, { cyclesFix: false }));
      console.log(
        colors.red(
          "Consider comitting or otherwise saving your work and pulling to load any remote changes",
        ),
      );
      error = true;
      return false;
    }
    return true;
  });
  if (error) {
    return;
  }

  for await (
    const { path, isIgnored, stateFile, localFile } of state.getFiles()
  ) {
    if (!path.endsWith(".json") || isIgnored) continue;

    const fileJSON = JSON.parse(localFile ?? "{}");
    const file = inferTypeFromPath(path, fileJSON);
    const eContent = (inferTypeFromPath(path, await stateFile.getContent())) ??
      {};

    const fileHash = objectHash(file);
    const eHash = objectHash(eContent);

    if (fileHash !== eHash) {
      const remotePath = stateFile.path.split(".")[0];
      const type = getTypeStrFromPath(stateFile.path);
      if (type === "script") {
        // Diffing makes no sense for scripts - instead fetch parent hash & check hash again.
        // If hash is still missmatched - create new script as child.
        const typed = decoverto.type(ScriptFile).plainToInstance(file);
        const contentPath = await findContentFile(stateFile.path);
        const language = inferContentTypeFromFilePath(contentPath);
        const content = await Deno.readTextFile(contentPath);
        try {
          const remote = await ScriptService.getScriptByPath({
            workspace: workspace.workspaceId,
            path: remotePath,
          });
          if (objectHash(remote) !== fileHash) {
            await ScriptService.createScript({
              workspace: workspace.workspaceId,
              requestBody: {
                content,
                description: typed.description,
                language,
                path: remotePath,
                summary: typed.summary,
                is_template: typed.is_template,
                kind: typed.kind,
                lock: undefined,
                parent_hash: remote.hash,
                schema: typed.schema,
              },
            });
          }
        } catch {
          // no parent hash
          await ScriptService.createScript({
            workspace: workspace.workspaceId,
            requestBody: {
              content,
              description: typed.description,
              language,
              path: remotePath,
              summary: typed.summary,
              is_template: typed.is_template,
              kind: typed.kind,
              lock: undefined,
              parent_hash: undefined,
              schema: typed.schema,
            },
          });
        }
      } else {
        const diff = microdiff(eContent as any, file, { cyclesFix: false });
        await applyDiff(
          workspace.workspaceId,
          remotePath,
          file,
          diff,
        );
      }
    }
  }

  let anyRemoteChanges = false;
  await updateStateFromRemote(workspace, state, (_) => {
    anyRemoteChanges = true;
    return true;
  });
  if (anyRemoteChanges) {
    console.log("New remote changes - consider pulling");
  }

  function applyDiff(
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
    return file.pushDiffs(workspace, remotePath, diffs);
  }
}

class StateDiff {
  localPath: string;
  diff: Difference[];

  constructor(
    localPath: string,
    diff: Difference[],
  ) {
    this.localPath = localPath;
    this.diff = diff;
  }
}

async function init(opts: GlobalOptions) {
  try {
    await Deno.mkdir(State.getInternalWmillFolder());
  } catch {
    console.log(
      colors.red(
        "! Looks like this folder is already initialized or we are missing permissions to do so. Exiting.",
      ),
    );
    return;
  }

  await Deno.writeTextFile(
    ".wmillignore",
    "# Write any ignores here as you see fit. Same syntax as .gitignore.",
  );

  const workspace = await resolveWorkspace(opts);

  const newState = new State(
    new Map(),
    new Map(),
    workspace.workspaceId,
    workspace.remote,
  );

  await newState.save();
}

async function pullRaw(
  opts: GlobalOptions & { override: boolean },
) {
  const workspace = await resolveWorkspace(opts);

  const zipDir = await downloadZip(workspace);
  if (!zipDir) return;

  // TODO use ZipFSElement & readDirRecursiveWithIgnore here
  // TODO also remember to read content via entry methods now instead of direct I/O
  for await (
    const entry of readDirRecursiveWithIgnore(
      (_) => false,
      ZipFSElement(zipDir),
    )
  ) {
    const filePath = entry.path;
    if (entry.isDirectory) {
      await ensureDir(filePath);
      continue;
    }
    await ensureDir(path.dirname(filePath));
    if (!opts.override) {
      let exists = false;
      try {
        const _stat = await Deno.stat(filePath);
        exists = true;
      } catch {
        exists = false;
      }
      if (exists) {
        if (
          !(await Confirm.prompt(
            "Conflict at " +
              filePath +
              " do you want to override the local version?",
          ))
        ) {
          continue;
        }
      }
    }
    await Deno.writeFile(filePath, await entry.getContentBytes());
  }
  console.log(colors.green("Done. Wrote all files to disk."));
}

type PushRawCandidate = {
  path: string;
  namespaceKind: "user" | "group" | "folder";
  namespaceName: string;
};

type PushRawResourceTypeCandidate = {
  path: string;
};

type PushRawFolderCandidate = {
  path: string;
  namespaceName: string;
};

async function pushRawFindCandidateFiles(
  dir: string,
): Promise<
  {
    normal: PushRawCandidate[];
    resourceTypes: PushRawResourceTypeCandidate[];
    folders: PushRawFolderCandidate[];
  }
> {
  dir = path.resolve(dir);
  if (path.dirname(dir).startsWith(".")) {
    return { normal: [], resourceTypes: [], folders: [] };
  }
  const normalCandidates: PushRawCandidate[] = [];
  const resourceTypeCandidates: PushRawResourceTypeCandidate[] = [];
  const folderCandidates: PushRawFolderCandidate[] = [];
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
        const { normal, resourceTypes, folders } =
          await pushRawFindCandidateFiles(
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

async function pushRaw(opts: GlobalOptions, dir?: string) {
  dir = dir ?? Deno.cwd();
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log(colors.blue("Searching Directory..."));
  const { normal, resourceTypes, folders } = await pushRawFindCandidateFiles(
    dir,
  );
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
      await typed.push(workspace.workspaceId, remotePath);
    }
  }
  console.log(colors.underline.bold.green("Successfully Pushed all files."));
}

const command = new Command()
  .command("init")
  .description(
    "Initialize this folder as sync root for the currently selected workspace & remote." +
      "\nBegin by initializing state tracking using `init` & add files you want to track using `add`. `push` & `pull` will then use local state to accurately track changes required on the remote.",
  )
  .action(init as any)
  .command("pull")
  .description(
    "Pull any remote changes and apply them locally. Use --raw for usage without local state tracking.",
  )
  .option("--raw", "Pull without using state.")
  .option("--raw-override", "Always override local files with remote.", {
    depends: ["raw"],
  })
  .action(pull as any)
  .command("push")
  .description(
    "Push any local changes and apply them remotely. Use --raw for usage without local state tracking.",
  )
  .option("--raw", "Push without using state.")
  .action(push as any);

export default command;
