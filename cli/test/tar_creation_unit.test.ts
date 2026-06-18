/**
 * Unit tests for the tar creation utility.
 * These tests require no backend — they test standalone tar logic.
 */

import { expect, test, describe } from "bun:test";
import { createTarBlob, type TarEntry } from "../src/utils/tar.ts";
import { extract, type Headers } from "tar-stream";
import { Readable } from "node:stream";

/** Extract all entries from a tarball Blob into a map of name -> content string */
async function extractTar(
  blob: Blob
): Promise<Map<string, { content: string; header: Headers }>> {
  const result = new Map<string, { content: string; header: Headers }>();
  const ex = extract();
  const buffer = Buffer.from(await blob.arrayBuffer());

  return new Promise((resolve, reject) => {
    ex.on("entry", (header, stream, next) => {
      const chunks: Buffer[] = [];
      stream.on("data", (chunk: Buffer) => chunks.push(chunk));
      stream.on("end", () => {
        result.set(header.name, {
          content: Buffer.concat(chunks).toString("utf-8"),
          header,
        });
        next();
      });
      stream.on("error", reject);
      stream.resume();
    });
    ex.on("finish", () => resolve(result));
    ex.on("error", reject);

    Readable.from(buffer).pipe(ex);
  });
}

describe("createTarBlob", () => {
  test("single file tarball", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: 'console.log("hello");' },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    expect(extracted.size).toBe(1);
    expect(extracted.has("main.js")).toBe(true);
    expect(extracted.get("main.js")!.content).toBe('console.log("hello");');
  });

  test("multiple output files", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: 'import "./chunk-abc.js";' },
      { name: "chunk-abc.js", content: "export const x = 42;" },
      { name: "chunk-def.js", content: "export const y = 99;" },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    expect(extracted.size).toBe(3);
    expect(extracted.get("main.js")!.content).toBe(
      'import "./chunk-abc.js";'
    );
    expect(extracted.get("chunk-abc.js")!.content).toBe(
      "export const x = 42;"
    );
    expect(extracted.get("chunk-def.js")!.content).toBe(
      "export const y = 99;"
    );
  });

  test("single file with assets", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: "const data = require('./data.json');" },
      { name: "data.json", content: '{"key":"value"}' },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    expect(extracted.size).toBe(2);
    expect(extracted.has("main.js")).toBe(true);
    expect(extracted.has("data.json")).toBe(true);
    expect(extracted.get("data.json")!.content).toBe('{"key":"value"}');
  });

  test("produces a valid Blob", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: "module.exports = {};" },
    ];

    const blob = await createTarBlob(entries);

    expect(blob).toBeInstanceOf(Blob);
    expect(blob.size).toBeGreaterThan(0);
    // Tar blocks are 512-byte aligned
    expect(blob.size % 512).toBe(0);
  });

  test("file naming — entries have exact names given", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: "entry point" },
      { name: "lib/utils.js", content: "utils" },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    // Names should be exactly as provided (no leading slash)
    expect(extracted.has("main.js")).toBe(true);
    expect(extracted.has("lib/utils.js")).toBe(true);
  });

  test("handles Buffer content", async () => {
    const entries: TarEntry[] = [
      { name: "main.js", content: Buffer.from("buffer content") },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    expect(extracted.get("main.js")!.content).toBe("buffer content");
  });

  test("handles Uint8Array content", async () => {
    const content = new TextEncoder().encode("uint8 content");
    const entries: TarEntry[] = [
      { name: "main.js", content },
    ];

    const blob = await createTarBlob(entries);
    const extracted = await extractTar(blob);

    expect(extracted.get("main.js")!.content).toBe("uint8 content");
  });
});
