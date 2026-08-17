import { describe, expect, it } from "bun:test";
import { createEvalArtifactHelpers } from "./evalArtifactStore";

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
});
