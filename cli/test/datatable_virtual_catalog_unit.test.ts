import { expect, test } from "bun:test";

import {
  queryTouchesVirtualDatabaseCatalog,
  virtualizeDatabaseCatalogQuery,
} from "../src/commands/datatable/virtual_catalog.ts";

test("queryTouchesVirtualDatabaseCatalog: detects current_database and pg_database", () => {
  expect(queryTouchesVirtualDatabaseCatalog("select current_database()")).toBe(true);
  expect(queryTouchesVirtualDatabaseCatalog("select * from pg_catalog.pg_database")).toBe(true);
  expect(queryTouchesVirtualDatabaseCatalog("select * from public.users")).toBe(false);
});

test("virtualizeDatabaseCatalogQuery: rewrites current_database to the virtual datatable name", () => {
  expect(
    virtualizeDatabaseCatalogQuery(
      "select current_database() as db",
      "sales-main",
      ["sales-main"],
    ),
  ).toBe("select 'sales-main' as db");
});

test("virtualizeDatabaseCatalogQuery: rewrites pg_database reads to the virtual datatable catalog", () => {
  const rewritten = virtualizeDatabaseCatalogQuery(
    "select datname from pg_catalog.pg_database where datname = current_database() order by datname",
    "sales-main",
    ["sales-main", "warehouse"],
  );

  expect(rewritten).toContain("FROM unnest(ARRAY['sales-main', 'warehouse']::text[]) WITH ORDINALITY");
  expect(rewritten).toContain("where datname = 'sales-main'");
  expect(rewritten).not.toContain("pg_catalog.pg_database");
});

test("virtualizeDatabaseCatalogQuery: neutralizes pg_database metadata helpers that need a real upstream database", () => {
  const rewritten = virtualizeDatabaseCatalogQuery(
    "select pg_size_pretty(pg_database_size(datname)) as size, has_database_privilege(datname, 'CONNECT') from pg_database",
    "sales-main",
    ["sales-main"],
  );

  expect(rewritten).toContain("NULL::text as size");
  expect(rewritten).toContain("true");
  expect(rewritten).not.toContain("pg_database_size");
});
