import { requireLogin, resolveWorkspace } from "./context.ts";
import { getWorkspaceStream, Workspace } from "./workspace.ts";
import { decoverto, map, MapShape, model, property } from "./decoverto.ts";
import {
  cbor,
  colors,
  Command,
  ensureDir,
  iterateReader,
  microdiff,
  nanoid,
  objectHash,
  path,
} from "./deps.ts";
import {
  Difference,
  GlobalOptions,
  inferTypeFromPath,
  setValueByPath,
} from "./types.ts";
import { downloadTar } from "./pull.ts";

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

  async getContent(): Promise<unknown> {
    if (this.#content_cache) {
      return this.#content_cache;
    }

    if (!this.#parent.stateRoot) {
      throw new Error("Parent uninitialized");
    }

    const f = this.#parent.contentFile(this.#id);
    if (!f) {
      throw new Error("Could not resolve content for self");
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
) {
  const untar = await downloadTar(workspace);
  if (!untar) throw new Error("Failed to pull Tar");

  const decoder = new TextDecoder();
  for await (const entry of untar) {
    const id = state.tracked.get(entry.fileName);
    if (id) {
      let val = "";
      for await (const e of iterateReader(entry)) {
        const tmp = decoder.decode(e);
        val += tmp;
      }
      const parsed = JSON.parse(val);

      const oldHash = state.hashes.get(id);
      const newHash = objectHash(parsed);

      if (!oldHash || oldHash !== newHash) {
        state.hashes.set(id, newHash);

        const encoded = CONTENT_ENCODER.encode(parsed);

        const fileName = nanoid();
        await Deno.writeFile(
          path.join(state.stateRoot!, ".wmill", fileName),
          encoded,
          { create: true },
        );
        state.contentFiles.set(id, fileName);
      }
    }
  }

  await state.save();
}

async function pull(opts: GlobalOptions) {
  const state = await getState(opts);

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  console.log("Pulling remote changes");
  await updateStateFromRemote(workspace, state);

  const diffs = await diffState(state);

  if (diffs.length > 0) {
    console.log(`Applying ${diffs.length} changes to files`);
    for (const diff of diffs) {
      await applyDiff(
        diff.diff,
        path.join(state.stateRoot!, diff.localPath),
      );
    }
    console.log(colors.green.underline("Done!"));
    // TODO: Commit changes here
  } else {
    console.log(colors.green.underline("everything is up to date"));
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

    // TODO: Use decoverto instanceToPlain below.
    await Deno.writeTextFile(file, JSON.stringify(json, undefined, "  "), {
      create: true,
    });
  }

  async function diffState(state: State): Promise<StateDiff[]> {
    const diffs: StateDiff[] = [];
    for (const t of state.tracked.keys()) {
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
          stateContent,
          { cyclesFix: false },
        );

        diffs.push(new StateDiff(entry.getId(), entry.path, diff));
      }
    }

    return diffs;
  }
}

async function push(opts: GlobalOptions) {
  const state = await getState(opts);

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await updateStateFromRemote(workspace, state);

  for (const p of state.tracked.keys()) {
    const entry = state.get(p);
    const file = JSON.parse(
      await Deno.readTextFile(path.join(state.stateRoot!, entry.path)),
    );
    const eContent = entry.getContent();

    const fileHash = objectHash(file);
    const eHash = objectHash(eContent);

    if (fileHash !== eHash) {
      const diff = microdiff(file, eContent, { cyclesFix: false });
      await applyDiff(entry.path, file, diff);
    }
  }

  await updateStateFromRemote(workspace, state);

  function applyDiff(path: string, file: any, diffs: Difference[]) {
    const typed = inferTypeFromPath(path, file);
    return typed.pushDiffs(diffs);
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

  await state.save();
}

async function sync(opts: GlobalOptions) {
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

const command = new Command()
  .description("Sync the current folder")
  .action(sync as any)
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
  .action(pull as any);

export default command;
