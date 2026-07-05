import { expect, test } from "bun:test";

import {
  detectMacroCalls,
  parseCreateMacro,
  parseMacroAnnotations,
  parseMacroLibrary,
  splitStatements,
} from "../src/commands/pipeline/duckdbMacros.ts";

// These mirror the accepted-shape cases of the backend parser
// (backend/parsers/windmill-parser/src/duckdb_macros.rs) so the local graph
// derives the same macro registry the server records at deploy.

test("parseCreateMacro: scalar, OR REPLACE / TEMP / TABLE, FUNCTION alias", () => {
  expect(parseCreateMacro("CREATE MACRO surrogate_key(a, b) AS md5(concat_ws('||', a, b))")).toEqual(
    { name: "surrogate_key", params: "a, b", isTable: false },
  );
  expect(parseCreateMacro("CREATE OR REPLACE TEMP MACRO top_n(t_max) AS TABLE SELECT * FROM t")).toEqual(
    { name: "top_n", params: "t_max", isTable: true },
  );
  // FUNCTION is DuckDB's alias for MACRO; name lowercased
  expect(parseCreateMacro("create function Dbl(a) as a * 2")).toEqual(
    { name: "dbl", params: "a", isTable: false },
  );
});

test("parseCreateMacro: default params + nested/string parens in the param list", () => {
  expect(
    parseCreateMacro("CREATE MACRO safe_div(a, b, fallback := (0)) AS CASE WHEN b = 0 THEN fallback ELSE a / b END")?.params,
  ).toBe("a, b, fallback := (0)");
  expect(parseCreateMacro("CREATE MACRO f(sep := '(') AS concat(sep, 'x')")?.params).toBe("sep := '('");
});

test("parseCreateMacro: rejects non-macro, qualified/quoted names, missing params/body", () => {
  expect(parseCreateMacro("SELECT 1")).toBeNull();
  expect(parseCreateMacro("CREATE TABLE t(x int)")).toBeNull();
  expect(parseCreateMacro("CREATE MACRO lake.m(a) AS a")).toBeNull();
  expect(parseCreateMacro('CREATE MACRO "weird name"(a) AS a')).toBeNull();
  expect(parseCreateMacro("CREATE MACRO m AS 1")).toBeNull(); // no params
  expect(parseCreateMacro("CREATE MACRO m(a) AS")).toBeNull(); // empty body
  expect(parseCreateMacro("CREATE OR MACRO m(a) AS a")).toBeNull(); // OR without REPLACE
});

test("splitStatements strips comments/strings and parseMacroLibrary skips setup", () => {
  const lib = `-- macros
ATTACH 'x.duckdb' AS ext;
CREATE MACRO m1(a) AS a; -- inline comment ; not a split
CREATE MACRO m2(b) AS b + 1;`;
  expect(splitStatements("SELECT 1; -- c ; still\nSELECT ';' AS x;").length).toBe(2);
  expect(parseMacroLibrary(lib).map((m) => m.name)).toEqual(["m1", "m2"]);
});

test("detectMacroCalls: word-boundary, case-insensitive, skips qualified/strings/comments", () => {
  const ns = new Set(["dbl", "avg_x"]);
  const found = detectMacroCalls("SELECT DBL(1), my_dbl(2), avg_x (3) FROM t", ns);
  expect([...found].sort()).toEqual(["avg_x", "dbl"]);
  expect(detectMacroCalls("SELECT lake.dbl(1)", new Set(["dbl"])).size).toBe(0);
  expect(detectMacroCalls("SELECT 'dbl(1)'", new Set(["dbl"])).size).toBe(0);
  expect(detectMacroCalls("-- dbl(1)\nSELECT 1", new Set(["dbl"])).size).toBe(0);
  expect(detectMacroCalls("SELECT dbl FROM t", new Set(["dbl"])).size).toBe(0); // no call parens
});

test("parseMacroAnnotations: leading-header only, marker stands alone, `// use` needs a path token", () => {
  expect(parseMacroAnnotations("-- macros\nCREATE MACRO m(a) AS a;", "--")).toEqual({
    macros: true,
    useLibs: [],
  });
  // marker with trailing prose is not the macros marker
  expect(parseMacroAnnotations("-- macros are below\nSELECT 1;", "--").macros).toBe(false);
  // `// use` accumulates path-shaped tokens, dedups, and stops at the body
  expect(
    parseMacroAnnotations("-- pipeline\n-- use f/lib/a\n-- use f/lib/a\n-- use notapath\nSELECT 1;", "--").useLibs,
  ).toEqual(["f/lib/a"]);
  // an annotation after code is ignored (leading header only)
  expect(parseMacroAnnotations("SELECT 1;\n-- use f/lib/a", "--").useLibs).toEqual([]);
});
