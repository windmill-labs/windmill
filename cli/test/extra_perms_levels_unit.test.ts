import { expect, test, beforeEach } from "bun:test";
import { mockServices } from "./mock_services.ts";

const added: unknown[] = [];
const removed: unknown[] = [];
mockServices({
  addGranularAcls: async (req: { requestBody: unknown }) => {
    added.push(req.requestBody);
    return "";
  },
  removeGranularAcls: async (req: { requestBody: unknown }) => {
    removed.push(req.requestBody);
    return "";
  },
});

const { applyExtraPermsDiff } = await import("../src/core/extra_perms.ts");

beforeEach(() => {
  added.length = 0;
  removed.length = 0;
});

// A grant is the level it gives, whichever of its spellings a file or the server uses.
test("booleans and level names of the same grant are in sync", async () => {
  const calls = await applyExtraPermsDiff(
    "ws",
    "script",
    "f/a/s",
    { "u/a": true, "u/b": "run", "u/c": "read" },
    { "u/a": "write", "u/b": false, "u/c": "read" },
  );
  expect(calls).toBe(0);
});

test("view-only is granted by level where it exists, as a read grant elsewhere", async () => {
  await applyExtraPermsDiff("ws", "flow", "f/a/f", { "u/a": "read" }, { "u/a": false });
  await applyExtraPermsDiff("ws", "variable", "f/a/v", { "u/b": "read" }, { "u/b": false });
  await applyExtraPermsDiff("ws", "variable", "f/a/v", { "u/c": "run" }, {});
  expect(added).toEqual([
    { owner: "u/a", write: false, level: "read" },
    { owner: "u/c", write: false },
  ]);
});
