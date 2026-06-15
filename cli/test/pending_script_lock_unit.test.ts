/**
 * Pending-lock preservation (issue #9588)
 *
 * A git-sync deploy mirror must not delete a committed `.script.lock` when the
 * server reports the script's lock as NULL — that NULL is transient (a relock
 * is mid-flight), not a real "no dependencies" state. A genuinely lock-free
 * script serializes `lock: ''` and must still drop its obsolete lock.
 */

import { expect, test } from "bun:test";
import { stringify as yamlStringify } from "yaml";
import { yamlParseContent } from "../src/utils/yaml.ts";
import { preservePendingScriptLocks } from "../src/commands/sync/sync.ts";

const META = "f/foo/bar.script.yaml";
const LOCK = "f/foo/bar.script.lock";
const LOCK_REF = "!inline f/foo/bar.script.lock";
const LOCK_CONTENT = "//deps\nsome-lock-content\n";

function localMaps() {
  return {
    [META]: yamlStringify({ summary: "bar", lock: LOCK_REF }),
    [LOCK]: LOCK_CONTENT,
  };
}

test("NULL remote lock preserves the committed lock file and metadata line", () => {
  // Remote during the race: lock key absent (NULL), no lock file emitted.
  const remote: Record<string, string> = {
    [META]: yamlStringify({ summary: "bar" }),
  };
  const local = localMaps();

  preservePendingScriptLocks(remote, local);

  // Lock file is carried onto the remote, so the diff sees no deletion.
  expect(remote[LOCK]).toBe(LOCK_CONTENT);
  // Metadata regains the inline lock reference, so no metadata edit either.
  const parsed = yamlParseContent(META, remote[META]) as Record<string, unknown>;
  expect(parsed["lock"]).toBe(LOCK_REF);
});

test("empty-string remote lock still removes the obsolete committed lock", () => {
  // Dependencies genuinely removed: lock is '' (present, computed, no deps).
  const remote: Record<string, string> = {
    [META]: yamlStringify({ summary: "bar", lock: "" }),
  };
  const local = localMaps();

  preservePendingScriptLocks(remote, local);

  // Unchanged: no lock file injected, so the diff still deletes the local lock.
  expect(remote[LOCK]).toBeUndefined();
  const parsed = yamlParseContent(META, remote[META]) as Record<string, unknown>;
  expect(parsed["lock"]).toBe("");
});

test("NULL lock without a committed local lock file is a no-op", () => {
  const remote: Record<string, string> = {
    [META]: yamlStringify({ summary: "bar" }),
  };
  // Local has the metadata but no committed lock file (nothing to preserve).
  const local: Record<string, string> = {
    [META]: yamlStringify({ summary: "bar", lock: LOCK_REF }),
  };

  preservePendingScriptLocks(remote, local);

  expect(remote[LOCK]).toBeUndefined();
});

test("deleted remote script does not resurrect its lock", () => {
  // Script removed remotely: no remote metadata key at all.
  const remote: Record<string, string> = {};
  const local = localMaps();

  preservePendingScriptLocks(remote, local);

  expect(remote[LOCK]).toBeUndefined();
  expect(remote[META]).toBeUndefined();
});
