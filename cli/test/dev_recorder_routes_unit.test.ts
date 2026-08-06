/**
 * Guards on the routes `wmill app dev --recording` adds: what may write a
 * recording, what a recording may be named, and that two saves never collide.
 */

import { expect, test } from "bun:test";
import {
  isOwnOrigin,
  isRecordingFileName,
  recordingFileName,
} from "../src/commands/app/devRecorder.ts";

test("only the shell's own origin may save a recording", () => {
  expect(isOwnOrigin("http://localhost:4000", "localhost:4000")).toBe(true);
  expect(isOwnOrigin("http://127.0.0.1:4000", "127.0.0.1:4000")).toBe(true);
  // A cross-site POST carrying JSON under a simple content type needs no
  // preflight, so a foreign origin sharing the port must still be refused.
  expect(isOwnOrigin("http://attacker.example:4000", "localhost:4000")).toBe(
    false,
  );
  expect(isOwnOrigin("null", "localhost:4000")).toBe(false);
  // No Origin at all is a non-browser client, not a cross-site page.
  expect(isOwnOrigin(undefined, "localhost:4000")).toBe(true);
});

test("recording names stay inside the recordings folder", () => {
  expect(isRecordingFileName("recording-2026-01-01-00-00-00-000.json")).toBe(
    true,
  );
  expect(isRecordingFileName("../../../etc/passwd")).toBe(false);
  expect(isRecordingFileName("..%2Fx.json")).toBe(false);
  expect(isRecordingFileName("sub/dir.json")).toBe(false);
  expect(isRecordingFileName("recording.txt")).toBe(false);
});

test("two saves in the same millisecond get distinct names", () => {
  const now = new Date("2026-01-01T00:00:00.123Z");
  const first = recordingFileName(now, 0);
  const second = recordingFileName(now, 1);
  expect(first).toBe("recording-2026-01-01-00-00-00-123.json");
  expect(second).not.toBe(first);
  expect(isRecordingFileName(second)).toBe(true);
});
