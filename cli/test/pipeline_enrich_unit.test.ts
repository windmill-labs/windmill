import { expect, test } from "bun:test";
import { recoverHeaderMarkers } from "../src/commands/pipeline/pipeline.ts";

// The deployed graph omits marker-only `// on data_upload`/`webhook`/`email`
// triggers, so `pipeline run` recovers them from script bodies. It must scan the
// LEADING comment header only (like the canonical/fallback parsers) so a body
// comment can't inject a phantom trigger and over-cut the run selection.

test("recovers marker triggers from the leading comment header", () => {
  expect(recoverHeaderMarkers("-- pipeline\n-- on data_upload\nSELECT 1;")).toEqual([
    "data_upload",
  ]);
  // multiple kinds, blank lines allowed inside the header
  expect(
    recoverHeaderMarkers("// pipeline\n\n// on webhook\n// on email\nexport function main(){}"),
  ).toEqual(["webhook", "email"]);
});

test("ignores a marker in a body comment (stops at first non-comment line)", () => {
  const code = `-- pipeline
-- on schedule
SELECT * FROM t;
-- on data_upload`;
  // the body `-- on data_upload` is past the header → not recovered
  expect(recoverHeaderMarkers(code)).toEqual([]);
});

test("ignores non-marker `on` kinds and dedupes", () => {
  // kafka/asset `on` lines aren't marker kinds; data_upload deduped
  const code = `# pipeline
# on data_upload
# on kafka
# on data_upload
def main(): pass`;
  expect(recoverHeaderMarkers(code)).toEqual(["data_upload"]);
});
