/**
 * `wmill app dev --recording` writes multi-MB session recordings into
 * `<app>.raw_app/recordings/`. They are local artifacts: the sync differ must
 * not offer them as app source (the push itself drops them in
 * `collectAppFiles`, so a differ that still sees them reports a change that
 * pushing can never settle).
 */

import { expect, test } from "bun:test";
import { sep as SEP } from "node:path";
import { elementsToMap } from "../src/commands/sync/sync.ts";

type MockFile = { path: string; content: string };

// FSFSElement joins with the platform separator, and the exclusion has to hold
// on Windows too.
const p = (...parts: string[]) => parts.join(SEP);

function mockElement(files: MockFile[]) {
  return {
    isDirectory: true,
    path: "",
    async getContentText() {
      return "";
    },
    async *getChildren() {
      for (const file of files) {
        yield {
          isDirectory: false,
          path: file.path,
          async getContentText() {
            return file.content;
          },
          async *getChildren() {},
        };
      }
    },
  };
}

test("elementsToMap skips recordings/ at the root of a raw app folder only", async () => {
  const app = p("f", "demo", "myapp.raw_app");
  const files: MockFile[] = [
    { path: p(app, "index.tsx"), content: "export {}" },
    {
      path: p(app, "recordings", "recording-2026-01-01-00-00-00.json"),
      content: '{"version":1}',
    },
    // The dev server never writes here, so this is the app's own source.
    { path: p(app, "src", "recordings", "fixture.json"), content: "{}" },
  ];

  const result = await elementsToMap(
    mockElement(files) as any,
    () => false,
    false,
    {},
  );

  expect(Object.keys(result).sort()).toEqual(
    [p(app, "index.tsx"), p(app, "src", "recordings", "fixture.json")].sort(),
  );
});
