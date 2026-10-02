import { afterAll, beforeAll, expect, test } from "bun:test";
import JSZip from "jszip";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  readDirRecursiveWithIgnore,
  ZipFSElement,
} from "../src/commands/sync/sync.ts";
import { itemDigest } from "../src/commands/digest/digest.ts";
import vectors from "./fixtures/item_digest_vectors.json";

// The server hashes the same vectors in backend/tests/job_provenance.rs, so a checkout
// `wmill sync pull` writes for them must digest to the values the job OIDC claims carry.

const originalCwd = process.cwd();
beforeAll(() => {
  process.chdir(mkdtempSync(join(tmpdir(), "wmill-digest-")));
});
afterAll(() => {
  process.chdir(originalCwd);
});

/** The workspace export of the vectors, as `GET .../workspaces/tarball` writes it. */
function exportZip(): JSZip {
  const zip = new JSZip();
  const file = (path: string, content: string) =>
    zip.file(path, content, { createFolders: false });
  const { script, flow } = vectors;
  file(`${script.path}.ts`, script.content);
  file(
    `${script.path}.script.json`,
    JSON.stringify({
      summary: "",
      description: "",
      schema: {},
      lock: script.lock,
      kind: "script",
      modules: script.modules,
    })
  );
  file(
    `${flow.path}.flow.json`,
    JSON.stringify({ summary: "", description: "", value: flow.value, schema: {} })
  );
  return zip;
}

async function writeTree(el: any) {
  for await (const e of readDirRecursiveWithIgnore(() => false, el)) {
    if (!e.isDirectory) {
      mkdirSync(dirname(e.path), { recursive: true });
      writeFileSync(e.path, await e.getContentText());
    }
  }
}

beforeAll(async () => {
  await writeTree(ZipFSElement(exportZip(), true, "bun", {}, {}, false, true));
});

// On Windows the pull leaves a multi-file script's entry point outside its `__mod/` folder
// (`ZipFSElement` matches module bases against `/`-separated zip names), a layout neither
// push nor this command reads back.
test.skipIf(process.platform === "win32")(
  "a pulled script digests to the server's digest",
  async () => {
    const script = await itemDigest("f/digest/script__mod/script.ts", "bun");
    expect(script).toEqual({
      path: vectors.script.path,
      kind: "script",
      digest: vectors.script.digest,
    });
    expect(
      (await itemDigest("f/digest/script__mod/script.yaml", "bun")).digest
    ).toBe(vectors.script.digest);
  }
);

test("a pulled flow digests to the server's digests", async () => {
  const flow = await itemDigest(join("f", "digest", "flow.flow"), "bun");
  expect(flow).toEqual({
    path: vectors.flow.path,
    kind: "flow",
    digest: vectors.flow.digest,
    steps: vectors.flow.steps,
  });
});
