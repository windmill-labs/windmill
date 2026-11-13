import { expect, test, beforeEach } from "bun:test";
import { mockServices } from "./mock_services.ts";

const added: unknown[] = [];
mockServices({
  addGranularAcls: async (req: { requestBody: unknown }) => {
    added.push(req.requestBody);
    return "";
  },
  removeGranularAcls: async () => "",
});

const { applyExtraPermsDiff } = await import("../src/core/extra_perms.ts");

beforeEach(() => {
  added.length = 0;
});

// A yaml written before view-only grants existed has no `view_only`: it must neither promote a
// remote view-only grant to run nor demote a new grant below run.
test("an absent view_only leaves levels alone", async () => {
  await applyExtraPermsDiff(
    "ws",
    "script",
    "f/a/s",
    { "u/viewer": false, "u/new": false },
    { "u/viewer": false },
    undefined,
    ["u/viewer"],
  );
  expect(added).toEqual([{ owner: "u/new", write: false }]);
});

test("view_only moves a read grant between view and run", async () => {
  await applyExtraPermsDiff(
    "ws",
    "flow",
    "f/a/f",
    { "u/a": false, "u/b": false },
    { "u/a": false, "u/b": false },
    ["u/a"],
    ["u/b"],
  );
  expect(added).toEqual([
    { owner: "u/a", write: false, level: "read" },
    { owner: "u/b", write: false, level: "run" },
  ]);
});
