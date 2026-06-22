/**
 * Unit tests for datatable-migration path parsing and local validation.
 *
 * These exercise pure logic with no backend:
 *  - `parseDatatableMigrationPath` recognizes only the
 *    `migrations/datatable/<dt>/<timestamp>_<name>.(up|down).sql` shape.
 *  - `validateLocalMigrations` rejects the two invalid on-disk states a push
 *    must catch: two up (or two down) files sharing a timestamp, and a
 *    `.down.sql` with no matching `.up.sql`.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { parseDatatableMigrationPath } from "../src/types.ts";
import { validateLocalMigrations } from "../src/commands/datatable_migrations.ts";

describe("parseDatatableMigrationPath", () => {
  test("parses up and down files of the new layout", () => {
    expect(
      parseDatatableMigrationPath(
        "migrations/datatable/mydt/20260101000001_create_users.up.sql",
      ),
    ).toEqual({
      datatable: "mydt",
      timestamp: 20260101000001,
      name: "create_users",
      kind: "up",
    });
    expect(
      parseDatatableMigrationPath(
        "migrations/datatable/my-dt/42_x.down.sql",
      ),
    ).toEqual({ datatable: "my-dt", timestamp: 42, name: "x", kind: "down" });
  });

  test("rejects unrelated, legacy and malformed paths", () => {
    for (
      const p of [
        // legacy top-level layout
        "datatable_migrations/mydt/20260101000001_x.up.sql",
        // wrong sub-namespace / depth
        "migrations/ducklake/mydt/1_x.up.sql",
        "migrations/datatable/1_x.up.sql",
        "migrations/datatable/mydt/sub/1_x.up.sql",
        // not a migration file
        "migrations/datatable/mydt/notes.txt",
        "migrations/datatable/mydt/x.up.sql", // no numeric timestamp prefix
        // unrelated workspace files
        "f/foo/bar.script.yaml",
        "u/admin/script.ts",
      ]
    ) {
      expect(parseDatatableMigrationPath(p)).toBeUndefined();
    }
  });
});

describe("validateLocalMigrations", () => {
  let prevCwd: string;
  let tmp: string;

  beforeEach(() => {
    prevCwd = process.cwd();
    tmp = fs.mkdtempSync(path.join(os.tmpdir(), "dtmig-"));
    process.chdir(tmp);
  });
  afterEach(() => {
    process.chdir(prevCwd);
    fs.rmSync(tmp, { recursive: true, force: true });
  });

  function write(datatable: string, file: string) {
    const dir = path.join(tmp, "migrations", "datatable", datatable);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, file), "-- sql\n");
  }

  test("accepts up+down pairs and up-only migrations", () => {
    write("mydt", "20260101000001_create_users.up.sql");
    write("mydt", "20260101000001_create_users.down.sql");
    write("mydt", "20260101000002_add_email.up.sql"); // down is optional
    expect(validateLocalMigrations()).toEqual([]);
  });

  test("flags two up files sharing a timestamp", () => {
    write("mydt", "20260101000003_foo.up.sql");
    write("mydt", "20260101000003_bar.up.sql");
    const errors = validateLocalMigrations();
    expect(errors.length).toBe(1);
    expect(errors[0]).toContain("20260101000003");
  });

  test("flags two down files sharing a timestamp", () => {
    write("mydt", "20260101000004_a.up.sql");
    write("mydt", "20260101000004_a.down.sql");
    write("mydt", "20260101000004_b.down.sql");
    const errors = validateLocalMigrations();
    // duplicate down + the b.down orphan (no b.up)
    expect(errors.some((e) => e.includes("down") && e.includes("20260101000004"))).toBe(true);
  });

  test("flags a down file with no matching up", () => {
    write("mydt", "20260101000005_orphan.down.sql");
    const errors = validateLocalMigrations();
    expect(errors.length).toBe(1);
    expect(errors[0]).toContain("20260101000005_orphan");
  });

  test("only validates the requested datatables", () => {
    write("bad", "20260101000006_x.up.sql");
    write("bad", "20260101000006_y.up.sql"); // duplicate, but in 'bad'
    write("good", "20260101000007_ok.up.sql");
    expect(validateLocalMigrations(new Set(["good"]))).toEqual([]);
    expect(validateLocalMigrations(new Set(["bad"])).length).toBe(1);
  });

  test("returns no errors when the migrations folder is absent", () => {
    expect(validateLocalMigrations()).toEqual([]);
  });
});
