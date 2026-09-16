import { describe, expect, it } from "bun:test";
import { buildWorkspaceId } from "./workspaceId";

describe("buildWorkspaceId", () => {
  // `workspace.proper_id` rejects `--`, which a case id can carry itself and
  // which truncating a slug on a hyphen produces once the suffix adds its own.
  // One id per shape: cut landing on a hyphen, cut landing mid-word, no cut, and
  // a doubled hyphen no cut ever reaches.
  it("stays within the id length cap and the proper_id format", () => {
    for (const caseId of [
      "global-test6-secret-variable-draft",
      "global-test23-datatable-query-select",
      "short",
      "global--test-foo",
    ]) {
      const id = buildWorkspaceId(caseId, 1);
      expect(id.length).toBeLessThanOrEqual(50);
      expect(id).toMatch(/^\w+(-\w+)*$/);
    }
  });
});
