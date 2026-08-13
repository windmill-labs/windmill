// SessionArtifactsStore can't run here (bun has no IndexedDB, nor the compiled $state runes),
// so mirror only the shape the artifact tools call, not its scoping or race handling.
export const EVAL_SESSION_ID = "eval-session";
export function createEvalArtifactHelpers() {
  const items = new Map<string, Record<string, any>>();
  // Snapshots per artifact id, oldest first — the version tools read history from here.
  const history = new Map<string, Array<Record<string, any>>>();
  let seq = 0;
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
      const now = seq++;
      const artifact = {
        id: `eval-artifact-${now}`,
        sessionId,
        chatId: input.chatId,
        kind: input.kind ?? "md",
        name: input.name,
        content: input.content,
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
      sessionId: EVAL_SESSION_ID,
      getChatId: () => "eval-chat",
      openArtifact: () => {},
    },
    snapshot: () => [...items.values()],
  };
}
