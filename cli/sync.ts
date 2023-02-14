import { requireLogin, resolveWorkspace } from "./context.ts";
import { getWorkspaceStream, Workspace } from "./workspace.ts";
import { decoverto, map, MapShape, model, property } from "./decoverto.ts";
import {
  cbor,
  colors,
  Command,
  Confirm,
  copy,
  ensureDir,
  iterateReader,
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
  #id: TrackedId;
  #parent: State;

  path: string;

  #content_cache?: string;

  constructor(id: TrackedId, parent: State, path: string) {
    this.#id = id;
    this.#parent = parent;
    this.path = path;
  }

  async getContent(): Promise<unknown | undefined> {
    if (this.#content_cache) {
      return this.#content_cache;
    }

    if (!this.#parent.stateRoot) {
      throw new Error("Parent uninitialized");
    }

    const f = this.#parent.contentFile(this.#id);
    if (!f) {
      return undefined;
    }

    const data = await Deno.readFile(
      path.join(this.#parent.stateRoot, ".wmill", f),
    );
    const content = CONTENT_ENCODER.decode(data);
    this.#content_cache = content;
    return content;
  }

  getHash(): string {
    return this.#parent.hashes.get(this.#id)!;
  }

  getId(): TrackedId {
    return this.#id;
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

  @property(map(() => String, () => TrackedId, { shape: MapShape.Object }))
  tracked: Map<string, TrackedId>;

  @property(() => String)
  workspaceId: string;

  @property(() => String)
  remoteUrl: string;

  stateRoot?: string;

  constructor(
    hashes: Map<TrackedId, string>,
    tracked: Map<string, TrackedId>,
    contentFiles: Map<TrackedId, string>,
    workspaceId: string,
    remoteUrl: string,
  ) {
    this.hashes = hashes;
    this.tracked = tracked;
    this.contentFiles = contentFiles;
    this.workspaceId = workspaceId;
    this.remoteUrl = remoteUrl;
  }

  add(path: string) {
    if (this.tracked.get(path)) {
      throw new Error("Cannot newly track already tracked paths");
    } else {
      this.tracked.set(path, nanoid());
    }
  }

  public forget(path: string) {
    const id = this.tracked.get(path);
    if (id) {
      this.tracked.delete(path);
      this.hashes.delete(id);
    }
  }

  public contentFile(id: TrackedId): string | undefined {
    return this.contentFiles.get(id);
  }

  public get(path: string): Tracked {
    const id = this.tracked.get(path);
    if (!id) {
      throw new Error("Could not resolve path " + path);
    }
    return new Tracked(id, this, path);
  }

  public async save(): Promise<void> {
    if (!this.stateRoot) {
      throw new Error("Uninitialized state root");
    }
    const encoder = new cbor.Encoder({});
    const plain = decoverto.type(State).instanceToPlain(this);
    const result: Uint8Array = encoder.encode(plain, {});
    await Deno.writeFile(path.join(this.stateRoot, ".wmill", "main"), result, {
      create: true,
    });
  }

  public static async loadState(dir: string): Promise<State> {
    const source = await Deno.readFile(path.join(dir, ".wmill", "main"));
    const encoder = new cbor.Encoder({});
    const raw = encoder.decode(source);
    const state = decoverto.type(State).plainToInstance(
      raw,
    );
    state.stateRoot = dir;
    return Object.freeze(state);
  }
}

async function getState(opts: GlobalOptions) {
  const existingState = await State.loadState(Deno.cwd());

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

  for await (const entry of Deno.readDir(zipDir)) {
    if (entry.isDirectory) continue;
    const id = state.tracked.get(entry.name);
    if (id) {
      const val = await Deno.readTextFile(path.resolve(zipDir, entry.name));
      if (entry.name.endsWith(".json")) {
        const parsed = JSON.parse(val);
        const typed = inferTypeFromPath(entry.name, parsed);

        const oldHash = state.hashes.get(id);
        const newHash = objectHash(typed);

        if (!oldHash || oldHash !== newHash) {
          if (!await callback(entry.name)) {
            return; // notice that we are not saving
          }
          state.hashes.set(id, newHash);

          const encoded = CONTENT_ENCODER.encode(typed);

          const fileName = nanoid();
          await Deno.writeFile(
            path.join(state.stateRoot!, ".wmill", fileName),
            encoded,
            { create: true },
          );
          state.contentFiles.set(id, fileName);
        }
      } else {
        const fileName = nanoid();
        await Deno.writeTextFile(
          path.join(state.stateRoot!, ".wmill", fileName),
          val,
          { create: true },
        );
        state.contentFiles.set(id, fileName);
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
    await pullRaw(opts2, Deno.cwd());
    return;
  }

  const state = await getState(opts);

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
      path.join(state.stateRoot!, diff.localPath),
    );
  }
  console.log(colors.green.underline("Done! All changes applied."));

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
    for (const t of state.tracked.keys()) {
      if (t.endsWith(".json")) {
        continue;
      }
      const entry = state.get(t);

      const target = await Deno.open(entry.path, { create: true, write: true });
      const source = await Deno.open(
        path.join(
          state.stateRoot!,
          ".wmill",
          state.contentFile(entry.getId())!,
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
  for (const t of state.tracked.keys()) {
    if (!t.endsWith(".json")) {
      continue;
    }
    const entry = state.get(t);
    let fileText;
    try {
      fileText = await Deno.readTextFile(
        path.join(state.stateRoot!, entry.path),
      );
    } catch {
      fileText = "{}";
    }
    const old = JSON.parse(fileText);
    const fileHash = objectHash(old);

    if (fileHash !== entry.getHash()) {
      const stateContent = await entry.getContent() as any;

      const diff = microdiff(
        old,
        stateContent ?? {},
        { cyclesFix: false },
      );

      yield new StateDiff(entry.getId(), entry.path, diff);
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
        await Deno.readTextFile(path.join(state.stateRoot!, e.path)),
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

  for (const p of state.tracked.keys()) {
    if (!p.endsWith(".json")) continue;
    const entry = state.get(p);
    let fileJSON;
    try {
      fileJSON = JSON.parse(
        await Deno.readTextFile(path.join(state.stateRoot!, entry.path)),
      );
    } catch {
      fileJSON = {};
    }
    const file = inferTypeFromPath(entry.path, fileJSON);
    const eContent =
      (inferTypeFromPath(entry.path, await entry.getContent())) ?? {};

    const fileHash = objectHash(file);
    const eHash = objectHash(eContent);

    if (fileHash !== eHash) {
      const remotePath = entry.path.split(".")[0];
      const type = getTypeStrFromPath(entry.path);
      if (type === "script") {
        // Diffing makes no sense for scripts - instead fetch parent hash & check hash again.
        // If hash is still missmatched - create new script as child.
        const typed = decoverto.type(ScriptFile).plainToInstance(file);
        const contentPath = await findContentFile(entry.path);
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
  trackedId: TrackedId;
  localPath: string;
  diff: Difference[];

  constructor(
    trackedId: TrackedId,
    localPath: string,
    diff: Difference[],
  ) {
    this.trackedId = trackedId;
    this.localPath = localPath;
    this.diff = diff;
  }
}

async function add(opts: GlobalOptions, path: string) {
  const state = await getState(opts);

  // TODO: Automatically check whether this path exists either locally or on the remote
  state.add(path);

  if (path.endsWith(".script.json")) {
    try {
      const f = await findContentFile(path);
      state.add(f);
    } catch {
      const workspace = await resolveWorkspace(opts);
      await requireLogin(opts);
      try {
        const old = await ScriptService.getScriptByPath({
          workspace: workspace.workspaceId,
          path: path.split(".")[0],
        });
        if (old.language === "python3") {
          state.add(path.replace(".script.json", ".py"));
        } else if (old.language === "bash") {
          state.add(path.replace(".script.json", ".sh"));
        } else if (old.language === "deno") {
          state.add(path.replace(".script.json", ".ts"));
        } else if (old.language === "go") {
          state.add(path.replace(".script.json", ".go"));
        } else {
          throw new Error("Remote returned invalid language?! " + old.language);
        }
      } catch {
        throw new Error(
          "Could not infer script language from local or remote. Exiting.",
        );
      }
    }
  }

  await state.save();
}

async function init(opts: GlobalOptions) {
  const root = Deno.cwd();
  try {
    await Deno.mkdir(path.join(root, ".wmill"));
  } catch {
    console.log(
      colors.red(
        "! Looks like this folder is already initialized or we are missing permissions to do so. Exiting.",
      ),
    );
    return;
  }

  const workspace = await resolveWorkspace(opts);

  const newState = new State(
    new Map(),
    new Map(),
    new Map(),
    workspace.workspaceId,
    workspace.remote,
  );
  newState.stateRoot = root;

  await newState.save();
}

async function pullRaw(
  opts: GlobalOptions & { override: boolean },
  dir: string,
) {
  const workspace = await resolveWorkspace(opts);

  const zipDir = await downloadZip(workspace);
  if (!zipDir) return;

  for await (const entry of Deno.readDir(zipDir)) {
    const filePath = path.resolve(dir, entry.name);
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
    await Deno.copyFile(path.resolve(zipDir, entry.name), filePath);
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
      typed.push(workspace.workspaceId, remotePath);
    }
  }
  console.log(colors.underline.bold.green("Successfully Pushed all files."));
}

const command = new Command()
  .command("init")
  .description(
    "Initialize this folder as sync root for the currently selected workspace & remote.",
  )
  .action(init as any)
  .command("add")
  .description("Add a local file for tracking")
  .arguments("<path:string>")
  .action(add as any)
  .command("pull")
  .description("Pull any remote changes and apply them locally")
  .option("--raw", "Pull without using state.")
  .option("--raw-override", "Always override local files with remote.", {
    depends: ["raw"],
  })
  .action(pull as any)
  .command("push")
  .description("Push any local changes and apply them remotely")
  .option("--raw", "Push without using state.")
  .action(push as any);

export default command;
