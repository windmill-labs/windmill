/**
 * `wmill app dev --recording` writes multi-MB session recordings into
 * `<app>.raw_app/recordings/`. They are local artifacts: the sync differ must
 * not offer them as app source (the push itself drops them in
 * `collectAppFiles`, so a differ that still sees them reports a change that
 * pushing can never settle).
 */

import { expect, test } from "bun:test";
import { elementsToMap } from "../src/commands/sync/sync.ts";

type MockFile = { path: string; content: string };

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

test("elementsToMap skips recordings/ inside a raw app folder", async () => {
  const files: MockFile[] = [
    { path: "f/demo/myapp.raw_app/index.tsx", content: "export {}" },
    {
      path: "f/demo/myapp.raw_app/recordings/recording-2026-01-01-00-00-00.json",
      content: '{"version":1}',
    },
  ];

  const result = await elementsToMap(
    mockElement(files) as any,
    () => false,
    false,
    {},
  );

  expect(Object.keys(result)).toEqual(["f/demo/myapp.raw_app/index.tsx"]);
});
