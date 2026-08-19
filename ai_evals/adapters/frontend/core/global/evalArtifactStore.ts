import { planArtifactId } from "../../../../../frontend/src/lib/components/copilot/chat/artifacts/planIdentity";

// SessionArtifactsStore can't run here (bun has no IndexedDB, nor the compiled $state runes),
// so mirror only the shape the artifact tools call, not its scoping or race handling.
// Cases run concurrently in one process and the preview handlers are registered
// process-wide, keyed by session id — so each run needs its own.
let sessionSeq = 0;

/** An artifact the session already holds when the case starts: history has to predate the
 * run, since one prompt cannot both build a past and reason about it. */
export interface SeededArtifact {
  name: string;
  role?: "plan";
  /** Which version the user agreed to. Below the last one means the current text is a
   * proposal they turned down, which is the state worth seeding. */
  approvedVersion?: number;
  /** Oldest first; the last one is the artifact's current content. */
  versions: Array<{ content: string; note?: string }>;
}

export function createEvalArtifactHelpers(seed: SeededArtifact[] = []) {
  const sessionId = `eval-session-${sessionSeq++}`;
  const items = new Map<string, Record<string, any>>();
  // Snapshots per artifact id, oldest first — the version tools read history from here.
  const history = new Map<string, Array<Record<string, any>>>();
  // How a preview-tab fixture names the artifact its tab shows.
  const seededIds = new Map<string, string>();
  let seq = 0;
  for (const entry of seed) {
    // Derived, not minted: the tools that must not touch the plan recognise it by this id, so
    // an id of the harness's own would pass a case the real gate refuses. The counter advances
    // either way, or seeding a plan would renumber the rows around it and collapse the update
    // order they are sorted on.
    const n = seq++;
    const id = entry.role === "plan" ? planArtifactId(sessionId) : `eval-artifact-${n}`;
    const current = entry.versions.at(-1);
    if (!current) continue;
    // A preview tab names the artifact it shows, so a shared name would open whichever
    // one happened to be seeded last.
    if (seededIds.has(entry.name)) {
      throw new Error(
        `Two seeded artifacts are named "${entry.name}" — a preview tab fixture could not tell them apart`,
      );
    }
    seededIds.set(entry.name, id);
    items.set(id, {
      id,
      sessionId,
      chatId: "eval-chat",
      kind: "md",
      name: entry.name,
      content: current.content,
      role: entry.role,
      approvedVersion: entry.approvedVersion,
      createdAt: 0,
      updatedAt: seq,
      version: entry.versions.length,
    });
    history.set(
      id,
      entry.versions.map((v, i) => ({
        key: `${id}:${i + 1}`,
        artifactId: id,
        version: i + 1,
        name: entry.name,
        content: v.content,
        savedAt: i,
        note: v.note,
      })),
    );
  }
  const snapshotOf = (
    artifact: Record<string, any>,
    version: number,
    note?: string,
  ) => ({
    key: `${artifact.id}:${version}`,
    artifactId: artifact.id,
    version,
    name: artifact.name,
    content: artifact.content,
    savedAt: artifact.updatedAt,
    note,
  });
  const store = {
    create: async (sessionId: string, input: Record<string, any>) => {
      // One plan per session, as SessionArtifactsStore enforces it — the tool refuses
      // first, so reaching this means a case drove create_artifact past that message.
      if (
        input.role === "plan" &&
        [...items.values()].some(
          (a) => a.sessionId === sessionId && a.role === "plan",
        )
      ) {
        throw new Error(`Session ${sessionId} already has a plan document`);
      }
      const now = seq++;
      const artifact = {
        id:
          input.role === "plan"
            ? planArtifactId(sessionId)
            : `eval-artifact-${now}`,
        sessionId,
        chatId: input.chatId,
        kind: input.kind ?? "md",
        name: input.name,
        content: input.content,
        // The plan document is only distinguishable by these, both in the snapshot the
        // judge reads and in what list_artifacts reports back to the model.
        role: input.role,
        approvedVersion: input.approvedVersion,
        createdAt: now,
        updatedAt: now,
        version: 1,
      };
      items.set(artifact.id, artifact);
      history.set(artifact.id, [snapshotOf(artifact, 1)]);
      return artifact;
    },
    get: async (id: string) => items.get(id),
    update: async (
      id: string,
      input: Record<string, any>,
      opts?: { sessionId?: string },
    ) => {
      const existing = items.get(id);
      if (!existing) return undefined;
      if (
        opts?.sessionId !== undefined &&
        existing.sessionId !== opts.sessionId
      )
        return undefined;
      // Only a content change earns a version, as in SessionArtifactsStore.
      const contentChanged =
        input.content !== undefined && input.content !== existing.content;
      const version = (existing.version ?? 1) + (contentChanged ? 1 : 0);
      const updated = {
        ...existing,
        name: input.name ?? existing.name,
        content: input.content ?? existing.content,
        // Carried only onto a version this write produced, as SessionArtifactsStore does:
        // a rename cannot promote a proposal the user turned down.
        approvedVersion:
          input.approvedVersion ??
          (input.keepApproved &&
          existing.approvedVersion !== undefined &&
          contentChanged
            ? version
            : existing.approvedVersion),
        updatedAt: seq++,
        version,
      };
      items.set(id, updated);
      if (contentChanged) {
        history.set(id, [
          ...(history.get(id) ?? []),
          snapshotOf(updated, version, input.note),
        ]);
      }
      return updated;
    },
    remove: async (id: string) => {
      items.delete(id);
      history.delete(id);
    },
    listForSession: async (sessionId: string) =>
      [...items.values()].filter((a) => a.sessionId === sessionId),
    listVersions: async (id: string) =>
      [...(history.get(id) ?? [])].sort((a, b) => b.version - a.version),
    getVersion: async (id: string, version: number) =>
      (history.get(id) ?? []).find((v) => v.version === version),
  };
  return {
    helpers: {
      artifacts: store,
      sessionId,
      getChatId: () => "eval-chat",
      openArtifact: (_id: string, _name: string) => {},
    },
    sessionId,
    seededIds,
    snapshot: () => [...items.values()],
  };
}
