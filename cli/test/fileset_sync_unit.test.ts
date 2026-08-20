/**
 * Unit tests for fileset resource sync routing and pointer validation.
 * These tests require no backend — they test standalone logic.
 */

import { expect, test, describe } from "bun:test";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, sep as SEP } from "node:path";
import { handleFile } from "../src/commands/script/script.ts";
import { validateFilesetPointer } from "../src/commands/resource/resource.ts";
import { findFilesetResourceFile } from "../src/commands/sync/sync.ts";
import {
  isCurrentWorkspaceFile,
  isWorkspaceSpecificFile,
} from "../src/core/specific_items.ts";

describe("handleFile routing", () => {
  test("returns false for fileset children with script extensions", async () => {
    // A fileset child belongs to its parent resource's value, never to a
    // standalone script: bare `.sql` has no script language (only `.pg.sql`
    // etc. do), so routing it to the script pusher aborts the whole push.
    for (const p of [
      "f/resources/data.fileset/energy/queries/report.sql",
      "f/resources/data.fileset/scripts/main.ts",
      "f/resources/data.fileset/scripts/main.py",
    ]) {
      expect(
        await handleFile(p, {} as any, [], undefined, undefined, {}, []),
      ).toBe(false);
    }
  });

  test("returns false for single-file resource content files", async () => {
    expect(
      await handleFile(
        "f/resources/query.resource.file.sql",
        {} as any,
        [],
        undefined,
        undefined,
        {},
        [],
      ),
    ).toBe(false);
  });
});

describe("workspace-specific classification", () => {
  test("fileset children are never workspace-specific", () => {
    // The workspace-name segment of the pattern spans `/`, so a child whose
    // own name looks like a typed metadata file would otherwise be read as
    // belonging to another workspace and dropped from every diff — i.e.
    // silently never deployed.
    for (const p of [
      "f/resources/data.fileset/edge/q.resource.file.sql",
      "f/resources/data.fileset/edge/inner.resource.yaml",
      "f/resources/data.fileset/queries/report.sql",
    ]) {
      expect(isWorkspaceSpecificFile(p)).toBe(false);
    }
    // A child carrying the active workspace's own suffix must not be remapped
    // onto the unsuffixed sibling key, which would deploy over that sibling.
    expect(
      isCurrentWorkspaceFile(
        "f/resources/data.fileset/edge/inner.ws_main.resource.yaml",
        "ws_main",
      ),
    ).toBe(false);
    // The parent's own metadata still carries the suffix.
    expect(isWorkspaceSpecificFile("f/resources/data.ws_main.resource.yaml")).toBe(
      true,
    );
    expect(
      isCurrentWorkspaceFile("f/resources/data.ws_main.resource.yaml", "ws_main"),
    ).toBe(true);
  });
});

describe("validateFilesetPointer", () => {
  test("accepts the server-canonical pointer", () => {
    expect(() =>
      validateFilesetPointer("f/resources/data.fileset", "f/resources/data"),
    ).not.toThrow();
  });

  test("normalizes trailing slashes", () => {
    expect(() =>
      validateFilesetPointer("f/resources/data.fileset/", "f/resources/data"),
    ).not.toThrow();
  });

  test("rejects a custom pointer with an actionable message", () => {
    expect(() =>
      validateFilesetPointer(
        "f/queries.fileset",
        "f/resources/analytics_data",
      ),
    ).toThrow(/must live next to its resource file, at 'f\/resources\/analytics_data\.fileset'/);
  });

  test("resolves workspace-specific metadata for canonical children", async () => {
    // The metadata file carries the workspace suffix while children stay at
    // the server-canonical `<base>.fileset/` directory.
    const dir = mkdtempSync(join(tmpdir(), "fileset-ws-"));
    const cwd = process.cwd();
    try {
      mkdirSync(join(dir, "f/res"), { recursive: true });
      writeFileSync(
        join(dir, "f/res/data.ws_main.resource.yaml"),
        "resource_type: c_files\nvalue: '!inline_fileset f/res/data.fileset'\n",
      );
      process.chdir(dir);
      // findFilesetResourceFile derives the metadata path from the child
      // path, so both use the platform separator.
      const childPath = ["f", "res", "data.fileset", "q.sql"].join(SEP);
      const wsMetadataPath = ["f", "res", "data.ws_main.resource.yaml"].join(
        SEP,
      );
      const baseMetadataPath = ["f", "res", "data.resource.yaml"].join(SEP);
      expect(await findFilesetResourceFile(childPath, "ws_main")).toBe(
        wsMetadataPath,
      );
      await expect(findFilesetResourceFile(childPath, null)).rejects.toThrow(
        /No resource metadata file found/,
      );
      // The suffixed file is the workspace's authoritative metadata: it wins
      // even when a base metadata file coexists with it.
      writeFileSync(
        join(dir, "f/res/data.resource.yaml"),
        "resource_type: c_files\nvalue: '!inline_fileset f/res/data.fileset'\n",
      );
      expect(await findFilesetResourceFile(childPath, "ws_main")).toBe(
        wsMetadataPath,
      );
      expect(await findFilesetResourceFile(childPath, null)).toBe(
        baseMetadataPath,
      );
    } finally {
      process.chdir(cwd);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("rejects a branch-suffixed directory for a workspace-specific resource", () => {
    // The remote exporter always renders children at the server-canonical
    // location, so a `.ws_<branch>.fileset` directory would never round-trip.
    expect(() =>
      validateFilesetPointer(
        "f/resources/data.ws_main.fileset",
        "f/resources/data",
      ),
    ).toThrow(/at 'f\/resources\/data\.fileset'/);
  });
});
