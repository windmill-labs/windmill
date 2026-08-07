import { describe, expect, it } from "bun:test";
import { createEvalArtifactHelpers } from "./evalArtifactStore";

// This store is a hand-written stand-in for SessionArtifactsStore (bun has no IndexedDB).
// It drifted once already: the version tools shipped and the stand-in still had no
// listVersions/getVersion, so `list_artifact_versions` threw mid-eval. Pin the surface
// and semantics the artifact tools rely on so the next divergence fails here instead.
describe("eval artifact store", () => {
  it("exposes every method the artifact tools call", () => {
    const { helpers } = createEvalArtifactHelpers();
    for (const method of [
      "create",
      "get",
      "update",
      "remove",
      "listForSession",
      "listVersions",
      "getVersion",
    ]) {
      expect(typeof (helpers.artifacts as any)[method]).toBe("function");
    }
  });

  it("versions a content change, records its note, and keeps the superseded text", async () => {
    const { helpers } = createEvalArtifactHelpers();
    const store = helpers.artifacts;

    const a = await store.create(helpers.sessionId, { name: "Plan", content: "v1 body" });
    await store.update(
      a.id,
      { content: "v2 body", note: "Added a rollback section" },
      { sessionId: helpers.sessionId },
    );

    // Newest first, and only the edit carries a note — a first version supersedes nothing.
    const versions = await store.listVersions(a.id);
    expect(versions.map((v: any) => [v.version, v.content, v.note])).toEqual([
      [2, "v2 body", "Added a rollback section"],
      [1, "v1 body", undefined],
    ]);
    expect((await store.getVersion(a.id, 1))?.content).toBe("v1 body");
    expect((await store.get(a.id))?.version).toBe(2);
  });

  it("does not version a rename or an identical rewrite", async () => {
    const { helpers } = createEvalArtifactHelpers();
    const store = helpers.artifacts;

    const a = await store.create(helpers.sessionId, { name: "Plan", content: "body" });
    await store.update(a.id, { name: "Renamed" }, { sessionId: helpers.sessionId });
    await store.update(a.id, { content: "body" }, { sessionId: helpers.sessionId });

    expect(await store.listVersions(a.id)).toHaveLength(1);
    expect((await store.get(a.id))?.version).toBe(1);
  });
});
