import { describe, expect, it } from "bun:test";
import { buildWorkspaceId } from "./workspaceId";

describe("buildWorkspaceId", () => {
  // `workspace.proper_id` rejects `--`, which truncating a slug on a hyphen
  // produces once the suffix adds its own. One id per shape the cut can take:
  // landing on a hyphen, landing mid-word, and short enough not to be cut.
  it("stays within the id length cap and the proper_id format", () => {
    for (const caseId of [
      "global-test6-secret-variable-draft",
      "global-test23-datatable-query-select",
      "short",
    ]) {
      const id = buildWorkspaceId(caseId, 1);
      expect(id.length).toBeLessThanOrEqual(50);
      expect(id).toMatch(/^\w+(-\w+)*$/);
    }
  });
});
