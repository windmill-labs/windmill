/**
 * The recorder `wmill app dev --recording` serves is generated from the
 * frontend's raw-app recorder, not written here, so it can silently ship a stale
 * event model after that recorder changes. The committed bundle records the hash
 * of the sources it was built from; this fails when they no longer agree.
 *
 * Fix a failure with `bun run gen:dev-recorder` from cli/.
 */

import { describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import {
  hashRecorderSources,
  RECORDER_SOURCES,
} from "../generate-dev-recorder.ts";
import {
  DEV_RECORDER_BUNDLE,
  DEV_RECORDER_SOURCE_HASH,
} from "../src/commands/app/devRecorderBundle.gen.ts";

const RECORDING_DIR = path.join(
  import.meta.dir,
  "..",
  "..",
  "frontend",
  "src",
  "lib",
  "components",
  "recording",
);

describe("dev recorder bundle", () => {
  test("exposes the recorder factory as a global", () => {
    expect(DEV_RECORDER_BUNDLE).toContain("__wmillRecorder");
    expect(DEV_RECORDER_BUNDLE).toContain("createRawAppRecording");
    // Runes are stripped at generation; one left in would throw at load time.
    expect(DEV_RECORDER_BUNDLE).not.toContain("$state");
  });

  test("is up to date with the frontend recorder", () => {
    const present = RECORDER_SOURCES.every((f) =>
      fs.existsSync(path.join(RECORDING_DIR, f))
    );
    // The published CLI package ships without the frontend sources.
    if (!present) return;
    expect(hashRecorderSources(RECORDING_DIR)).toBe(DEV_RECORDER_SOURCE_HASH);
  });
});
