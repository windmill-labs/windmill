import { expect, test } from "bun:test";

import {
  findCaseInsensitiveCollisions,
  canonicalizeCaseInsensitiveKeys,
  summarizeCaseRewrites,
} from "../src/commands/sync/sync.ts";

// =============================================================================
// Case-insensitive sync handling (WIN-2020)
//
// Windmill paths are case-sensitive, but Windows and the default macOS setup
// use case-insensitive filesystems. The real-world failure is NOT a user
// deliberately authoring both f/Caps and f/caps — it is a single capitalized
// folder whose on-disk casing drifts (Windows reports the case the directory
// was first created with), so the diff sees a brand-new lowercase path and a
// destructive delete of the real one. These tests pin:
//   1. findCaseInsensitiveCollisions  — warn about genuinely unrepresentable
//      server-side collisions (two distinct remote paths differing only by
//      case).
//   2. canonicalizeCaseInsensitiveKeys — the fix: rewrite drifted local keys
//      to the server's casing so no phantom delete+add is produced.
// =============================================================================

function normalize(groups: string[][]): string[][] {
  return groups
    .map((g) => [...g])
    .sort((a, b) => a.join().localeCompare(b.join()));
}

// ---------------------------------------------------------------------------
// findCaseInsensitiveCollisions
// ---------------------------------------------------------------------------

test("collision detection: sibling folders differing only by case", () => {
  const collisions = findCaseInsensitiveCollisions([
    "f/Caps/a.script.ts",
    "f/caps/b.script.ts",
  ]);
  expect(normalize(collisions)).toEqual([["f/Caps", "f/caps"]]);
});

test("collision detection: leaf files differing only by case", () => {
  const collisions = findCaseInsensitiveCollisions([
    "f/team/Report.script.ts",
    "f/team/report.script.ts",
  ]);
  expect(normalize(collisions)).toEqual([
    ["f/team/Report.script.ts", "f/team/report.script.ts"],
  ]);
});

test("collision detection: none for case-consistent distinct paths", () => {
  const collisions = findCaseInsensitiveCollisions([
    "f/caps/foo.script.ts",
    "f/caps/bar.script.ts",
    "f/other/baz.script.ts",
  ]);
  expect(collisions).toEqual([]);
});

test("collision detection: normalizes mixed forward/back slashes", () => {
  const collisions = findCaseInsensitiveCollisions([
    "f\\Caps\\a.script.ts",
    "f/caps/b.script.ts",
  ]);
  expect(normalize(collisions)).toEqual([["f/Caps", "f/caps"]]);
});

test("collision detection: reports only the shallowest clash for same-named leaves", () => {
  // Two case-variant folders that ALSO hold a same-named file must report the
  // single folder clash, not the folder group plus a nested per-file group.
  const collisions = findCaseInsensitiveCollisions([
    "f/Caps/main.script.ts",
    "f/caps/main.script.ts",
  ]);
  expect(normalize(collisions)).toEqual([["f/Caps", "f/caps"]]);
});

test("collision detection: groups three distinct casings together", () => {
  const collisions = findCaseInsensitiveCollisions([
    "f/caps/a.script.ts",
    "f/CAPS/b.script.ts",
    "f/Caps/c.script.ts",
  ]);
  expect(normalize(collisions)).toEqual([["f/CAPS", "f/Caps", "f/caps"]]);
});

// ---------------------------------------------------------------------------
// canonicalizeCaseInsensitiveKeys (the WIN-2020 fix)
// ---------------------------------------------------------------------------

test("canonicalize: drifted local folder casing adopts the server casing", () => {
  // Server (remote) is authoritative: f/Caps. The local tree drifted to
  // f/caps on a case-insensitive FS.
  const remote = {
    "f/Caps/x.script.ts": "content",
    "f/Caps/x.script.yaml": "meta",
  };
  const local = {
    "f/caps/x.script.ts": "content",
    "f/caps/x.script.yaml": "meta",
  };
  const { map, ambiguous, rewritten } = canonicalizeCaseInsensitiveKeys(
    local,
    remote,
  );
  // Local keys are rewritten to the server casing, so a subsequent exact-key
  // diff sees identical paths — no phantom delete+add.
  expect(Object.keys(map).sort()).toEqual([
    "f/Caps/x.script.ts",
    "f/Caps/x.script.yaml",
  ]);
  expect(ambiguous).toEqual([]);
  expect(rewritten).toEqual([
    { from: "f/caps/x.script.ts", to: "f/Caps/x.script.ts" },
    { from: "f/caps/x.script.yaml", to: "f/Caps/x.script.yaml" },
  ]);
});

test("canonicalize: preserves content values while rewriting keys", () => {
  const remote = { "f/MyFolder/Script.script.ts": "remote" };
  const local = { "f/myfolder/Script.script.ts": "LOCAL EDIT" };
  const { map } = canonicalizeCaseInsensitiveKeys(local, remote);
  expect(map["f/MyFolder/Script.script.ts"]).toEqual("LOCAL EDIT");
  expect(map["f/myfolder/Script.script.ts"]).toBeUndefined();
});

test("canonicalize: leaves keys with no case-insensitive remote match", () => {
  const remote = { "f/Caps/x.script.ts": "a" };
  const local = {
    "f/Caps/x.script.ts": "a",
    "f/brand_new/y.script.ts": "b", // genuinely local-only add
  };
  const { map, rewritten } = canonicalizeCaseInsensitiveKeys(local, remote);
  expect(rewritten).toEqual([]);
  expect(Object.keys(map).sort()).toEqual([
    "f/Caps/x.script.ts",
    "f/brand_new/y.script.ts",
  ]);
});

test("canonicalize: does NOT rewrite when the server casing is ambiguous", () => {
  // The server itself holds two paths differing only by case — we must not
  // silently pick one. Leave the local key untouched and report the ambiguity.
  const remote = {
    "f/Caps/x.script.ts": "a",
    "f/caps/x.script.ts": "b",
  };
  const local = { "f/CAPS/x.script.ts": "local" };
  const { map, ambiguous, rewritten } = canonicalizeCaseInsensitiveKeys(
    local,
    remote,
  );
  expect(rewritten).toEqual([]);
  expect(Object.keys(map)).toEqual(["f/CAPS/x.script.ts"]);
  // Reported as the shallowest (folder) clash, not the nested per-file group.
  expect(normalize(ambiguous)).toEqual([["f/Caps", "f/caps"]]);
});

test("canonicalize: new local file under a drifted folder adopts the server folder casing", () => {
  // Regression for the P1 review finding: a brand-new local file has no exact
  // remote match, but it still lives under a folder whose casing drifted. It
  // must inherit the server's folder casing (f/Caps), otherwise push would
  // create f/caps/New beside f/Caps/* and reintroduce the case-only collision.
  const remote = { "f/Caps/Existing.script.ts": "remote" };
  const local = {
    "f/caps/Existing.script.ts": "remote", // drifted, existing
    "f/caps/New.script.ts": "brand new", // drifted folder, local-only file
  };
  const { map, rewritten } = canonicalizeCaseInsensitiveKeys(local, remote);
  expect(Object.keys(map).sort()).toEqual([
    "f/Caps/Existing.script.ts",
    "f/Caps/New.script.ts",
  ]);
  expect(map["f/Caps/New.script.ts"]).toEqual("brand new");
  expect(rewritten).toContainEqual({
    from: "f/caps/New.script.ts",
    to: "f/Caps/New.script.ts",
  });
});

test("canonicalize: stops at the first segment with no server guidance", () => {
  // Only the folder prefix that exists on the server is canonicalized; deeper
  // local-only directories keep their own casing.
  const remote = { "f/Caps/x.script.ts": "a" };
  const local = { "f/caps/SubDir/y.script.ts": "b" };
  const { map } = canonicalizeCaseInsensitiveKeys(local, remote);
  expect(Object.keys(map)).toEqual(["f/Caps/SubDir/y.script.ts"]);
});

test("canonicalize: identical casing is a no-op", () => {
  const remote = { "f/Caps/x.script.ts": "a" };
  const local = { "f/Caps/x.script.ts": "a" };
  const { map, rewritten, ambiguous } = canonicalizeCaseInsensitiveKeys(
    local,
    remote,
  );
  expect(rewritten).toEqual([]);
  expect(ambiguous).toEqual([]);
  expect(map).toEqual({ "f/Caps/x.script.ts": "a" });
});

// ---------------------------------------------------------------------------
// summarizeCaseRewrites
// ---------------------------------------------------------------------------

test("summarize: collapses per-file rewrites into one folder entry", () => {
  const summary = summarizeCaseRewrites([
    { from: "f/caps/x.script.ts", to: "f/Caps/x.script.ts" },
    { from: "f/caps/x.script.yaml", to: "f/Caps/x.script.yaml" },
    { from: "f/caps/y.script.ts", to: "f/Caps/y.script.ts" },
  ]);
  expect(summary).toEqual(["f/caps -> f/Caps"]);
});

test("summarize: reports a leaf-file casing change at file granularity", () => {
  const summary = summarizeCaseRewrites([
    { from: "f/team/report.script.ts", to: "f/team/Report.script.ts" },
  ]);
  expect(summary).toEqual([
    "f/team/report.script.ts -> f/team/Report.script.ts",
  ]);
});

test("summarize: reports independent folder drifts separately", () => {
  const summary = summarizeCaseRewrites([
    { from: "f/caps/x.script.ts", to: "f/Caps/x.script.ts" },
    { from: "u/alice/y.script.ts", to: "u/Alice/y.script.ts" },
  ]);
  expect(summary.sort()).toEqual(["f/caps -> f/Caps", "u/alice -> u/Alice"]);
});
