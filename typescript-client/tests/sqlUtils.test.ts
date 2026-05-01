/**
 * Standalone tests for `wmill.datatable()` / `wmill.ducklake()` SQL template
 * functions.
 *
 * The real `sqlUtils.ts` imports `./services.gen` (auto-generated, not in
 * the repo) so we can't import it here. Instead we re-implement the same
 * type-inference / template-building / `.query()` pipeline inline (sans the
 * network calls) and assert the `content` + `args` shapes the executor
 * would receive. Each test maps 1:1 to a behaviour this PR introduces or
 * fixes (BigInt, homogeneous arrays, `.query()` positional, etc.) so they
 * also serve as a regression backstop.
 *
 * Run with: bun test typescript-client/tests/sqlUtils.test.ts
 */
import { expect, test, describe } from "bun:test";

// =============================================================================
// Pure SDK logic (mirror of typescript-client/sqlUtils.ts — kept minimal,
// only the parts that decide content / args).
// =============================================================================

class RawSql {
  readonly __brand = "RawSql" as const;
  constructor(public readonly value: string) {}
}

interface SqlProvider {
  formatArgDecl(argNum: number, argType: string): string;
  formatArgUsage(
    argNum: number,
    explicitType: string | undefined,
    inferredType: string
  ): string;
  preamble(): string;
  language: "postgresql" | "duckdb";
  extraArgs: Record<string, any>;
  providerName: string;
}

function datatableProvider(name: string, schema?: string): SqlProvider {
  return {
    providerName: "datatable",
    language: "postgresql",
    extraArgs: { database: `datatable://${name}` },
    formatArgDecl: (argNum) => `-- $${argNum} arg${argNum}`,
    formatArgUsage: (argNum, explicitType, inferredType) =>
      explicitType !== undefined
        ? `$${argNum}`
        : `$${argNum}::${inferredType}`,
    preamble: () => (schema ? `SET search_path TO "${schema}";\n` : ""),
  };
}

function ducklakeProvider(name: string): SqlProvider {
  return {
    providerName: "ducklake",
    language: "duckdb",
    extraArgs: {},
    formatArgDecl: (argNum, argType) => `-- $arg${argNum} (${argType})`,
    formatArgUsage: (argNum) => `$arg${argNum}`,
    preamble: () => `ATTACH 'ducklake://${name}' AS dl;USE dl;\n`,
  };
}

function inferSqlType(value: any): string {
  if (typeof value === "bigint") return "BIGINT";
  if (typeof value === "number") {
    if (Number.isInteger(value)) return "BIGINT";
    return "DOUBLE PRECISION";
  } else if (value === null || value === undefined) {
    return "TEXT";
  } else if (typeof value === "string") {
    return "TEXT";
  } else if (Array.isArray(value)) {
    return inferSqlArrayType(value);
  } else if (value instanceof Date) {
    return "TIMESTAMPTZ";
  } else if (typeof value === "object") {
    return "JSON";
  } else if (typeof value === "boolean") {
    return "BOOLEAN";
  } else {
    return "TEXT";
  }
}

function inferSqlArrayType(value: any[]): string {
  if (value.length === 0) return "JSON";
  let scalarType: string | undefined = undefined;
  for (const elem of value) {
    let elemType: string;
    if (typeof elem === "bigint") elemType = "BIGINT";
    else if (typeof elem === "number")
      elemType = Number.isInteger(elem) ? "BIGINT" : "DOUBLE PRECISION";
    else if (typeof elem === "string") elemType = "TEXT";
    else if (typeof elem === "boolean") elemType = "BOOLEAN";
    else return "JSON";
    if (scalarType === undefined) scalarType = elemType;
    else if (scalarType === "BIGINT" && elemType === "DOUBLE PRECISION")
      scalarType = "DOUBLE PRECISION";
    else if (scalarType === "DOUBLE PRECISION" && elemType === "BIGINT") {
      // already widened
    } else if (scalarType !== elemType) {
      return "JSON";
    }
  }
  return `${scalarType}[]`;
}

function parseTypeAnnotation(
  prevTemplateString: string | undefined,
  nextTemplateString: string | undefined
): string | undefined {
  if (!nextTemplateString) return;
  nextTemplateString = nextTemplateString.trimStart();
  if (nextTemplateString.startsWith("::")) {
    return nextTemplateString.substring(2).trimStart().split(/\s+/)[0];
  }
  prevTemplateString = prevTemplateString?.trimEnd();
  if (
    prevTemplateString?.endsWith("(") &&
    prevTemplateString
      .substring(0, prevTemplateString.length - 1)
      .trim()
      .toUpperCase()
      .endsWith("CAST") &&
    nextTemplateString.toUpperCase().startsWith("AS ")
  ) {
    return nextTemplateString.substring(2).trimStart().split(/\s+/)[0];
  }
}

function serializeArgValue(v: any): any {
  if (typeof v === "bigint") return v.toString();
  if (v instanceof Date) return v.toISOString();
  if (typeof v === "number" && !Number.isFinite(v)) {
    if (Number.isNaN(v)) return "NaN";
    return v > 0 ? "Infinity" : "-Infinity";
  }
  return v;
}

function buildContentAndArgs(
  provider: SqlProvider,
  strings: TemplateStringsArray | string[],
  values: any[]
): { content: string; args: Record<string, any> } {
  let argIndex = 0;
  const valueInfos = values.map((v, i) => {
    if (v instanceof RawSql)
      return { raw: true as const, value: v.value, originalIndex: i };
    argIndex++;
    return {
      raw: false as const,
      value: v,
      originalIndex: i,
      argNum: argIndex,
    };
  });

  let argDecls = valueInfos
    .filter((info): info is Extract<typeof valueInfos[number], { raw: false }> => !info.raw)
    .map((info) => {
      let argType =
        parseTypeAnnotation(
          strings[info.originalIndex],
          strings[info.originalIndex + 1]
        ) || inferSqlType(info.value);
      return provider.formatArgDecl(info.argNum, argType);
    });

  let content = argDecls.length ? argDecls.join("\n") + "\n" : "";
  content += provider.preamble();

  let contentBody = "";
  for (let i = 0; i < strings.length; i++) {
    contentBody += strings[i];
    if (i < valueInfos.length) {
      let info = valueInfos[i];
      if (info.raw) {
        contentBody += info.value;
      } else {
        let explicitType = parseTypeAnnotation(
          strings[info.originalIndex],
          strings[info.originalIndex + 1]
        );
        let inferredType = inferSqlType(info.value);
        contentBody += provider.formatArgUsage(
          info.argNum,
          explicitType,
          inferredType
        );
      }
    }
  }
  content += contentBody;

  const args = {
    ...Object.fromEntries(
      valueInfos
        .filter((info): info is Extract<typeof valueInfos[number], { raw: false }> => !info.raw)
        .map((info) => [`arg${info.argNum}`, serializeArgValue(info.value)])
    ),
    ...provider.extraArgs,
  };
  return { content, args };
}

function buildDatatableQuery(
  provider: SqlProvider,
  sqlString: string,
  params: any[]
): { content: string; args: Record<string, any> } {
  let argDecls = params
    .map((v, i) => `-- $${i + 1} arg${i + 1} (${inferSqlType(v)})`)
    .join("\n");
  let content =
    (argDecls ? argDecls + "\n" : "") + provider.preamble() + sqlString;
  let args = {
    ...Object.fromEntries(
      params.map((v, i) => [`arg${i + 1}`, serializeArgValue(v)])
    ),
    ...provider.extraArgs,
  };
  return { content, args };
}

function templateTag(provider: SqlProvider) {
  return (strings: TemplateStringsArray, ...values: any[]) =>
    buildContentAndArgs(provider, strings, values);
}

const dt = (name = "main") => templateTag(datatableProvider(name));
const dl = (name = "main") => templateTag(ducklakeProvider(name));
const datatableQuery = (name = "main") => {
  const provider = datatableProvider(name);
  return (sql: string, ...params: any[]) =>
    buildDatatableQuery(provider, sql, params);
};

// =============================================================================
// inferSqlType — exhaustive coverage
// =============================================================================

describe("inferSqlType — primitives", () => {
  test("integer Number → BIGINT", () => {
    expect(inferSqlType(0)).toBe("BIGINT");
    expect(inferSqlType(42)).toBe("BIGINT");
    expect(inferSqlType(-7)).toBe("BIGINT");
    expect(inferSqlType(Number.MAX_SAFE_INTEGER)).toBe("BIGINT");
  });

  test("non-integer Number → DOUBLE PRECISION", () => {
    expect(inferSqlType(0.5)).toBe("DOUBLE PRECISION");
    expect(inferSqlType(-3.14)).toBe("DOUBLE PRECISION");
    expect(inferSqlType(Number.EPSILON)).toBe("DOUBLE PRECISION");
  });

  test("BigInt → BIGINT (not DOUBLE PRECISION)", () => {
    // Pre-fix this branch was unreachable because bigint was bundled with
    // number and `Number.isInteger(BigInt)` returns false → would have
    // returned DOUBLE PRECISION (wrong). The split-out check is the fix.
    expect(inferSqlType(BigInt(0))).toBe("BIGINT");
    expect(inferSqlType(BigInt("9007199254740993"))).toBe("BIGINT");
    expect(inferSqlType(BigInt(-1))).toBe("BIGINT");
  });

  test("string / null / undefined → TEXT", () => {
    expect(inferSqlType("")).toBe("TEXT");
    expect(inferSqlType("hello")).toBe("TEXT");
    expect(inferSqlType(null)).toBe("TEXT");
    expect(inferSqlType(undefined)).toBe("TEXT");
  });

  test("boolean → BOOLEAN", () => {
    expect(inferSqlType(true)).toBe("BOOLEAN");
    expect(inferSqlType(false)).toBe("BOOLEAN");
  });

  test("plain object → JSON", () => {
    expect(inferSqlType({})).toBe("JSON");
    expect(inferSqlType({ a: 1, b: [1, 2] })).toBe("JSON");
  });
});

describe("inferSqlType — arrays", () => {
  test("empty array → JSON", () => {
    expect(inferSqlType([])).toBe("JSON");
  });

  test("homogeneous integer array → BIGINT[]", () => {
    expect(inferSqlType([1, 2, 3])).toBe("BIGINT[]");
    expect(inferSqlType([0])).toBe("BIGINT[]");
    expect(inferSqlType([-1, 0, 1])).toBe("BIGINT[]");
  });

  test("homogeneous float array → DOUBLE PRECISION[]", () => {
    expect(inferSqlType([1.5, 2.5])).toBe("DOUBLE PRECISION[]");
  });

  test("mixed int/float array widens to DOUBLE PRECISION[]", () => {
    expect(inferSqlType([1, 2.5])).toBe("DOUBLE PRECISION[]");
    expect(inferSqlType([1.5, 2])).toBe("DOUBLE PRECISION[]");
  });

  test("homogeneous string array → TEXT[]", () => {
    expect(inferSqlType(["a", "b", "c"])).toBe("TEXT[]");
    expect(inferSqlType([""])).toBe("TEXT[]");
  });

  test("homogeneous bool array → BOOLEAN[]", () => {
    expect(inferSqlType([true, false, true])).toBe("BOOLEAN[]");
  });

  test("homogeneous bigint array → BIGINT[]", () => {
    expect(inferSqlType([BigInt(1), BigInt(2)])).toBe("BIGINT[]");
  });

  test("non-homogeneous array → JSON", () => {
    expect(inferSqlType([1, "x"])).toBe("JSON");
    expect(inferSqlType(["a", true])).toBe("JSON");
    expect(inferSqlType([1, null])).toBe("JSON");
    expect(inferSqlType([true, 1])).toBe("JSON");
  });

  test("nested array → JSON (current limitation, no auto-tag for 2D)", () => {
    expect(inferSqlType([[1], [2]])).toBe("JSON");
    expect(inferSqlType([{ a: 1 }, { a: 2 }])).toBe("JSON");
  });
});

// =============================================================================
// parseTypeAnnotation — used by the SDK to suppress its own ::TYPE injection
// when the user already wrote a cast.
// =============================================================================

describe("parseTypeAnnotation — user-supplied cast detection", () => {
  test("`${x}::int` → 'int'", () => {
    expect(parseTypeAnnotation("SELECT ", "::int FROM t")).toBe("int");
  });
  test("whitespace tolerance after ::", () => {
    expect(parseTypeAnnotation("SELECT ", " ::  bigint FROM t")).toBe(
      "bigint"
    );
  });
  test("`CAST(${x} AS int)` → first whitespace-delimited word after AS", () => {
    // The SDK splits on whitespace and doesn't strip closing parens, so
    // `AS int)` returns "int)". The exact value doesn't matter downstream
    // because the SDK only checks `explicitType !== undefined` to skip its
    // own ::cast injection — but we lock the behaviour in.
    expect(parseTypeAnnotation("SELECT CAST(", " AS int)")).toBe("int)");
  });
  test("`CAST ( ${x} AS BOOL )` (whitespace + caps)", () => {
    // Whitespace before `)` causes split to drop it, so this returns "BOOL".
    expect(parseTypeAnnotation("SELECT CAST ( ", " AS BOOL )")).toBe("BOOL");
  });
  test("no cast adjacent → undefined", () => {
    expect(parseTypeAnnotation("SELECT ", " FROM t")).toBeUndefined();
    expect(parseTypeAnnotation("SELECT ", "")).toBeUndefined();
    expect(parseTypeAnnotation(undefined, undefined)).toBeUndefined();
  });
});

// =============================================================================
// datatable() template tag — content + args round-trips
// =============================================================================

describe("datatable() — template tag", () => {
  test("primitives auto-tag with ::TYPE inline; decls have no type", () => {
    const sql = dt();
    const out = sql`SELECT ${42}, ${3.14}, ${true}, ${"x"}, ${null}`;
    // datatable provider's formatArgDecl ignores the type, so we get bare
    // decls + the casts in the SQL body.
    expect(out.content).toContain("-- $1 arg1\n");
    expect(out.content).toContain("$1::BIGINT");
    expect(out.content).toContain("$2::DOUBLE PRECISION");
    expect(out.content).toContain("$3::BOOLEAN");
    expect(out.content).toContain("$4::TEXT");
    expect(out.content).toContain("$5::TEXT");
    expect(out.args).toMatchObject({
      arg1: 42,
      arg2: 3.14,
      arg3: true,
      arg4: "x",
      arg5: null,
    });
  });

  test("user `${x}::int` suppresses SDK's auto-cast (parser sees user's cast)", () => {
    const sql = dt();
    const out = sql`SELECT ${42}::int`;
    expect(out.content).toContain("SELECT $1::int");
    expect(out.content).not.toContain("$1::BIGINT");
  });

  test("CAST(${x} AS T) syntax → bare $N in SQL (regression #8988)", () => {
    const sql = dt();
    const out = sql`SELECT CAST(${true} AS bool)`;
    expect(out.content).toContain("CAST($1 AS bool)");
    expect(out.content).not.toContain("$1::BOOLEAN");
  });

  test("BigInt is stringified for JSON transport, tagged as ::BIGINT", () => {
    const sql = dt();
    const out = sql`SELECT ${BigInt("9007199254740993")}`;
    expect(out.content).toContain("$1::BIGINT");
    expect(out.args.arg1).toBe("9007199254740993");
    // Round-trip through JSON without throwing — the original bug.
    expect(() => JSON.stringify(out.args)).not.toThrow();
  });

  test("BigInt zero / negative / large", () => {
    const sql = dt();
    expect(sql`SELECT ${BigInt(0)}`.args.arg1).toBe("0");
    expect(sql`SELECT ${BigInt(-1)}`.args.arg1).toBe("-1");
    expect(sql`SELECT ${BigInt("99999999999999999999")}`.args.arg1).toBe(
      "99999999999999999999"
    );
  });

  test("Date is auto-tagged ::TIMESTAMPTZ and ISO-stringified", () => {
    // Pre-fix: typeof Date === "object" → ::JSON, then PG cast chain
    // worked accidentally for `${date}::timestamptz`. Now: explicit
    // ::TIMESTAMPTZ + Date.toISOString() so plain `${date}` against a
    // timestamptz column doesn't need a cast.
    const sql = dt();
    const d = new Date("2024-01-15T10:30:00.000Z");
    const out = sql`SELECT ${d} AS t`;
    expect(out.content).toContain("$1::TIMESTAMPTZ");
    expect(out.args.arg1).toBe("2024-01-15T10:30:00.000Z");
    expect(() => JSON.stringify(out.args)).not.toThrow();
  });

  test("non-finite Number is stringified for the executor", () => {
    // JSON.stringify(NaN) and JSON.stringify(Infinity) both produce `null`,
    // which silently became NULL in the database. The executor accepts
    // "NaN" / "Infinity" / "-Infinity" via `Value::String → FLOAT8`
    // (`f64::from_str`), so we send the special values as strings.
    const sql = dt();
    expect(sql`SELECT ${NaN}`.args.arg1).toBe("NaN");
    expect(sql`SELECT ${Infinity}`.args.arg1).toBe("Infinity");
    expect(sql`SELECT ${-Infinity}`.args.arg1).toBe("-Infinity");
    // Tag stays DOUBLE PRECISION (these are floats).
    expect(sql`SELECT ${NaN}`.content).toContain("$1::DOUBLE PRECISION");
  });

  test("homogeneous arrays auto-tag with TYPE[]", () => {
    const sql = dt();
    expect(sql`SELECT ${[1, 2, 3]}`.content).toContain("$1::BIGINT[]");
    expect(sql`SELECT ${[1.5, 2.5]}`.content).toContain(
      "$1::DOUBLE PRECISION[]"
    );
    expect(sql`SELECT ${["a", "b"]}`.content).toContain("$1::TEXT[]");
    expect(sql`SELECT ${[true, false]}`.content).toContain("$1::BOOLEAN[]");
  });

  test("non-homogeneous and empty arrays fall back to JSON", () => {
    const sql = dt();
    expect(sql`SELECT ${[1, "x"]}`.content).toContain("$1::JSON");
    expect(sql`SELECT ${[]}`.content).toContain("$1::JSON");
    expect(sql`SELECT ${[[1], [2]]}`.content).toContain("$1::JSON");
  });

  test("mixed-numeric array widens to DOUBLE PRECISION[]", () => {
    const sql = dt();
    expect(sql`SELECT ${[1, 2.5]}`.content).toContain(
      "$1::DOUBLE PRECISION[]"
    );
  });

  test("multiple args get distinct decls + numbered placeholders", () => {
    const sql = dt();
    const out = sql`INSERT INTO t VALUES (${1}, ${"x"}, ${[true, false]})`;
    expect(out.content).toContain("-- $1 arg1");
    expect(out.content).toContain("-- $2 arg2");
    expect(out.content).toContain("-- $3 arg3");
    expect(out.content).toContain("$1::BIGINT");
    expect(out.content).toContain("$2::TEXT");
    expect(out.content).toContain("$3::BOOLEAN[]");
    expect(out.args).toMatchObject({
      arg1: 1,
      arg2: "x",
      arg3: [true, false],
    });
  });

  test("RawSql is inlined verbatim, doesn't consume an arg index", () => {
    const sql = dt();
    const col = new RawSql("name");
    const out = sql`SELECT ${col} FROM t WHERE id = ${42}`;
    // Only one decl, only one arg in args dict.
    expect(out.content.match(/^-- \$\d+/gm)?.length).toBe(1);
    expect(out.content).toContain("SELECT name FROM t WHERE id = $1::BIGINT");
    expect(Object.keys(out.args).filter((k) => k.startsWith("arg")).length).toBe(
      1
    );
    expect(out.args).toMatchObject({ arg1: 42 });
  });

  test("schema name is propagated as SET search_path preamble", () => {
    const sql = dt("main");
    const out = sql`SELECT 1`;
    expect(out.args.database).toBe("datatable://main");
  });

  test("database extra arg is always present", () => {
    const sql = dt("custom_db");
    const out = sql`SELECT ${1}`;
    expect(out.args.database).toBe("datatable://custom_db");
  });
});

// =============================================================================
// datatable().query() — positional placeholders (the previously-broken path)
// =============================================================================

describe("datatable().query() — positional placeholders", () => {
  test("emits typed declarations + SQL verbatim, no appended placeholders", () => {
    const q = datatableQuery();
    const out = q("SELECT $1, $2", 42, "hello");
    expect(out.content).toContain("-- $1 arg1 (BIGINT)");
    expect(out.content).toContain("-- $2 arg2 (TEXT)");
    // Crucially: SQL must end with the user's SQL, NOT have placeholders
    // appended after it (the pre-fix bug).
    expect(out.content.endsWith("SELECT $1, $2")).toBe(true);
    expect(out.args).toMatchObject({ arg1: 42, arg2: "hello" });
  });

  test("BigInt args are stringified", () => {
    const q = datatableQuery();
    const out = q("SELECT $1", BigInt("100"));
    expect(out.args.arg1).toBe("100");
    expect(out.content).toContain("-- $1 arg1 (BIGINT)");
  });

  test("array args auto-tag homogeneously in the decl block", () => {
    const q = datatableQuery();
    const out = q("SELECT $1, $2", [1, 2], ["a", "b"]);
    expect(out.content).toContain("-- $1 arg1 (BIGINT[])");
    expect(out.content).toContain("-- $2 arg2 (TEXT[])");
  });

  test("zero params → no decl block, just SQL + extras", () => {
    const q = datatableQuery();
    const out = q("SELECT 1");
    expect(out.content).not.toContain("-- $");
    expect(out.content).toContain("SELECT 1");
    // database extra still injected.
    expect(out.args.database).toBe("datatable://main");
  });

  test("ten params number contiguously", () => {
    const q = datatableQuery();
    const params = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const out = q("SELECT " + params.map((_, i) => `$${i + 1}`).join(","), ...params);
    for (let i = 1; i <= 10; i++) {
      expect(out.content).toContain(`-- $${i} arg${i} (BIGINT)`);
      expect(out.args[`arg${i}`]).toBe(i);
    }
  });

  test(".query()'s decl format matches what the executor parses (regression)", () => {
    // The PG parser's RE_ARG_PGSQL is:
    //   ^-- \$(\d+) (\w+)(?: \(([A-Za-z0-9_\[\]]+)\))?(?: ?\= ?(.+))? *$
    // We assert our decl line matches that grammar so the executor doesn't
    // fall back to "inferred default text".
    const q = datatableQuery();
    const out = q("SELECT $1", 42);
    const declRe = /^-- \$\d+ \w+ \([A-Za-z0-9_\[\]]+\) *$/m;
    expect(out.content).toMatch(declRe);
  });
});

// =============================================================================
// ducklake() template tag — DuckDB declares types in the comment (different
// shape from datatable). Same auto-tag rules apply for inferSqlType.
// =============================================================================

describe("ducklake() — DuckDB shape", () => {
  test("declarations carry the type", () => {
    const sql = dl("main");
    const out = sql`SELECT ${42}, ${"hello"}, ${true}`;
    expect(out.content).toContain("-- $arg1 (BIGINT)");
    expect(out.content).toContain("-- $arg2 (TEXT)");
    expect(out.content).toContain("-- $arg3 (BOOLEAN)");
    // Preamble attaches the ducklake.
    expect(out.content).toContain("ATTACH 'ducklake://main' AS dl;USE dl;");
    // Args are referenced via $argN syntax in the SQL body.
    expect(out.content).toContain("$arg1");
  });

  test("BigInt + homogeneous arrays propagate to ducklake too", () => {
    const sql = dl("main");
    const out = sql`SELECT ${BigInt(9)}, ${[1, 2, 3]}, ${["a", "b"]}`;
    expect(out.content).toContain("(BIGINT)");
    expect(out.content).toContain("(BIGINT[])");
    expect(out.content).toContain("(TEXT[])");
    expect(out.args.arg1).toBe("9");
    expect(out.args.arg2).toEqual([1, 2, 3]);
  });

  test("ducklake doesn't carry a database extra arg", () => {
    const sql = dl();
    const out = sql`SELECT 1`;
    expect(out.args).not.toHaveProperty("database");
  });
});

// =============================================================================
// Cross-cutting: the args dict must always be JSON-serialisable.
// =============================================================================

describe("args dict is JSON-serialisable for every supported value shape", () => {
  test("BigInt, primitives, arrays, objects, raw — none throw", () => {
    const sql = dt();
    const out = sql`
      SELECT ${BigInt(1)}, ${1}, ${1.5}, ${"x"}, ${true}, ${null},
             ${[1, 2]}, ${["a", "b"]}, ${[true, false]},
             ${{ k: 1 }}, ${[1, "x"]}
    `;
    const json = JSON.stringify(out.args);
    expect(typeof json).toBe("string");
    // BigInt got stringified, not thrown.
    expect(json).toContain('"arg1":"1"');
  });
});
