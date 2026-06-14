import { expect, test } from "bun:test";
import {
  isDatatableMigrationPath,
  parseDatatableMigrationPath,
} from "../src/commands/datatable/migrations.ts";

test("isDatatableMigrationPath matches only .sql files under _datatable_migrations/", () => {
  expect(
    isDatatableMigrationPath("_datatable_migrations/main/20260101120000000_init.sql"),
  ).toBe(true);
  expect(
    isDatatableMigrationPath("_datatable_migrations/main/20260101120000000.sql"),
  ).toBe(true);
  // not .sql
  expect(
    isDatatableMigrationPath("_datatable_migrations/main/notes.txt"),
  ).toBe(false);
  // ordinary script .sql outside the folder
  expect(isDatatableMigrationPath("f/scripts/query.pg.sql")).toBe(false);
  // similarly-named folder elsewhere
  expect(
    isDatatableMigrationPath("f/_datatable_migrations/main/20260101120000000_init.sql"),
  ).toBe(false);
});

test("parseDatatableMigrationPath extracts datatable, version and name", () => {
  expect(
    parseDatatableMigrationPath("_datatable_migrations/main/20260101120000000_add_users.sql"),
  ).toEqual({ datatable: "main", version: "20260101120000000", name: "add_users" });
  // name is optional
  expect(
    parseDatatableMigrationPath("_datatable_migrations/main/20260101120000000.sql"),
  ).toEqual({ datatable: "main", version: "20260101120000000", name: "" });
  // name may contain underscores
  expect(
    parseDatatableMigrationPath("_datatable_migrations/dt/123_a_b_c.sql"),
  ).toEqual({ datatable: "dt", version: "123", name: "a_b_c" });
});

test("parseDatatableMigrationPath rejects malformed paths", () => {
  // version must be numeric
  expect(
    parseDatatableMigrationPath("_datatable_migrations/main/init.sql"),
  ).toBeUndefined();
  // missing datatable directory level
  expect(
    parseDatatableMigrationPath("_datatable_migrations/20260101120000000_init.sql"),
  ).toBeUndefined();
  // extra nesting
  expect(
    parseDatatableMigrationPath("_datatable_migrations/a/b/20260101120000000_init.sql"),
  ).toBeUndefined();
});
