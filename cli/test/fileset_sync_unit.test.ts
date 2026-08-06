/**
 * Unit tests for fileset resource sync routing and pointer validation.
 * These tests require no backend — they test standalone logic.
 */

import { expect, test, describe } from "bun:test";
import { handleFile } from "../src/commands/script/script.ts";
import { validateFilesetPointer } from "../src/commands/resource/resource.ts";

describe("handleFile routing", () => {
  test("returns false for fileset children with script extensions", async () => {
    // A fileset child is part of its parent resource's value; treating it as
    // a standalone script used to crash the push (`Invalid language: .sql`).
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
