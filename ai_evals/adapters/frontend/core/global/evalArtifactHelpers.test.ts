import { describe, expect, it } from "bun:test";
import { createEvalArtifactHelpers } from "./evalArtifactStore";
import { planArtifactId } from "../../../../../frontend/src/lib/components/copilot/chat/artifacts/planIdentity";

// A hand-written stand-in for SessionArtifactsStore (bun has no IndexedDB), so nothing
// makes it follow that class. A method missing from it surfaces as a tool throwing
// part-way through an eval run, which reads as a model failure rather than a harness one.
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

  it("files a plan under the id production derives, seeded or created", async () => {
    const { helpers, sessionId } = createEvalArtifactHelpers([
      { name: "Seeded plan", role: "plan", versions: [{ content: "v1" }] },
    ]);
    const seeded = await helpers.artifacts.listForSession(sessionId);
    expect(seeded.map((a: any) => a.id)).toEqual([planArtifactId(sessionId)]);

    const other = createEvalArtifactHelpers();
    const created = await other.helpers.artifacts.create(other.sessionId, {
      name: "Plan",
      content: "v1",
      role: "plan",
    });
    expect(created.id).toBe(planArtifactId(other.sessionId));
  });
});
