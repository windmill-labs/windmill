/**
 * Unit tests for pure utility functions.
 * These tests require no backend — they test standalone logic.
 */

import { expect, test, describe } from "bun:test";
import { deepEqual, isFileResource, toCamel, capitalize } from "../src/utils/utils.ts";
import {
  getTypeStrFromPath,
  removeType,
  isSuperset,
  extractNativeTriggerInfo,
  removePathPrefix,
} from "../src/types.ts";
import { validatePath } from "../src/core/context.ts";
import { inferContentTypeFromFilePath } from "../src/utils/script_common.ts";
import {
  filePathExtensionFromContentType,
  removeExtensionToPath,
} from "../src/commands/script/script.ts";

// =============================================================================
// deepEqual
// =============================================================================

describe("deepEqual", () => {
  test("primitives", () => {
    expect(deepEqual(1, 1)).toBe(true);
    expect(deepEqual(1, 2)).toBe(false);
    expect(deepEqual("a", "a")).toBe(true);
    expect(deepEqual("a", "b")).toBe(false);
    expect(deepEqual(true, true)).toBe(true);
    expect(deepEqual(true, false)).toBe(false);
    expect(deepEqual(null, null)).toBe(true);
    expect(deepEqual(undefined, undefined)).toBe(true);
    expect(deepEqual(null, undefined)).toBe(false);
  });

  test("NaN equality", () => {
    expect(deepEqual(NaN, NaN)).toBe(true);
    expect(deepEqual(NaN, 1)).toBe(false);
  });

  test("arrays", () => {
    expect(deepEqual([1, 2, 3], [1, 2, 3])).toBe(true);
    expect(deepEqual([1, 2, 3], [1, 2, 4])).toBe(false);
    expect(deepEqual([1, 2], [1, 2, 3])).toBe(false);
    expect(deepEqual([], [])).toBe(true);
  });

  test("nested arrays", () => {
    expect(deepEqual([[1, 2], [3]], [[1, 2], [3]])).toBe(true);
    expect(deepEqual([[1, 2], [3]], [[1, 2], [4]])).toBe(false);
  });

  test("objects", () => {
    expect(deepEqual({ a: 1, b: 2 }, { a: 1, b: 2 })).toBe(true);
    expect(deepEqual({ a: 1, b: 2 }, { a: 1, b: 3 })).toBe(false);
    expect(deepEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(deepEqual({}, {})).toBe(true);
  });

  test("nested objects", () => {
    expect(deepEqual({ a: { b: 1 } }, { a: { b: 1 } })).toBe(true);
    expect(deepEqual({ a: { b: 1 } }, { a: { b: 2 } })).toBe(false);
  });

  test("mixed nested structures", () => {
    const a = { arr: [1, { x: "hello" }], n: null };
    const b = { arr: [1, { x: "hello" }], n: null };
    expect(deepEqual(a, b)).toBe(true);

    const c = { arr: [1, { x: "world" }], n: null };
    expect(deepEqual(a, c)).toBe(false);
  });

  test("Maps", () => {
    const m1 = new Map([["a", 1], ["b", 2]]);
    const m2 = new Map([["a", 1], ["b", 2]]);
    const m3 = new Map([["a", 1], ["b", 3]]);
    expect(deepEqual(m1, m2)).toBe(true);
    expect(deepEqual(m1, m3)).toBe(false);
  });

  test("Sets", () => {
    const s1 = new Set([1, 2, 3]);
    const s2 = new Set([1, 2, 3]);
    const s3 = new Set([1, 2, 4]);
    expect(deepEqual(s1, s2)).toBe(true);
    expect(deepEqual(s1, s3)).toBe(false);
  });

  test("RegExp", () => {
    expect(deepEqual(/abc/g, /abc/g)).toBe(true);
    expect(deepEqual(/abc/g, /abc/i)).toBe(false);
    expect(deepEqual(/abc/, /def/)).toBe(false);
  });
});

// =============================================================================
// toCamel & capitalize
// =============================================================================

describe("toCamel", () => {
  test("converts snake_case to camelCase", () => {
    expect(toCamel("hello_world")).toBe("helloWorld");
    expect(toCamel("my_variable_name")).toBe("myVariableName");
  });

  test("converts kebab-case to camelCase", () => {
    expect(toCamel("hello-world")).toBe("helloWorld");
  });

  test("handles no separators", () => {
    expect(toCamel("hello")).toBe("hello");
  });
});

describe("capitalize", () => {
  test("capitalizes first character", () => {
    expect(capitalize("hello")).toBe("Hello");
    expect(capitalize("world")).toBe("World");
  });

  test("handles single character", () => {
    expect(capitalize("a")).toBe("A");
  });

  test("handles already capitalized", () => {
    expect(capitalize("Hello")).toBe("Hello");
  });

  test("handles empty string", () => {
    expect(capitalize("")).toBe("");
  });
});

// =============================================================================
// isFileResource
// =============================================================================

describe("isFileResource", () => {
  test("detects resource file paths", () => {
    expect(isFileResource("f/test/my_file.resource.file.txt")).toBe(true);
    expect(isFileResource("u/admin/config.resource.file.json")).toBe(true);
  });

  test("rejects non-resource-file paths", () => {
    expect(isFileResource("f/test/my_resource.resource.yaml")).toBe(false);
    expect(isFileResource("f/test/my_script.ts")).toBe(false);
    expect(isFileResource("f/test/my_flow.flow/flow.yaml")).toBe(false);
  });

  test("detects branch-specific resource file paths", () => {
    expect(isFileResource("f/test/config.main.resource.file.json")).toBe(true);
  });
});

// =============================================================================
// removeType
// =============================================================================

describe("removeType", () => {
  test("removes .variable.yaml suffix", () => {
    expect(removeType("f/test/my_var.variable.yaml", "variable")).toBe("f/test/my_var");
  });

  test("removes .resource.yaml suffix", () => {
    expect(removeType("f/test/my_res.resource.yaml", "resource")).toBe("f/test/my_res");
  });

  test("removes .schedule.yaml suffix", () => {
    expect(removeType("u/admin/cron.schedule.yaml", "schedule")).toBe("u/admin/cron");
  });

  test("removes .json suffix too", () => {
    expect(removeType("f/test/my_var.variable.json", "variable")).toBe("f/test/my_var");
  });

  test("throws for wrong type suffix", () => {
    expect(() => removeType("f/test/my_var.variable.yaml", "resource")).toThrow();
  });

  test("throws for no type suffix", () => {
    expect(() => removeType("f/test/my_script.ts", "variable")).toThrow();
  });
});

// =============================================================================
// removePathPrefix
// =============================================================================

describe("removePathPrefix", () => {
  test("removes prefix from path", () => {
    expect(removePathPrefix("f/test/my_script.ts", "f/test")).toBe("my_script.ts");
  });

  test("handles exact match", () => {
    expect(removePathPrefix("f/test", "f/test")).toBe("");
  });

  test("throws when prefix doesn't match", () => {
    expect(() => removePathPrefix("g/admin/script.ts", "f/test")).toThrow();
  });
});

// =============================================================================
// getTypeStrFromPath
// =============================================================================

describe("getTypeStrFromPath", () => {
  test("detects script types by extension", () => {
    expect(getTypeStrFromPath("f/test/my_script.ts")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.py")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.go")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.sh")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.sql")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.php")).toBe("script");
    expect(getTypeStrFromPath("f/test/my_script.rs")).toBe("script");
  });

  test("detects metadata types by name suffix", () => {
    expect(getTypeStrFromPath("f/test/my_var.variable.yaml")).toBe("variable");
    expect(getTypeStrFromPath("f/test/my_res.resource.yaml")).toBe("resource");
    expect(getTypeStrFromPath("f/test/my_sched.schedule.yaml")).toBe("schedule");
    expect(getTypeStrFromPath("f/test/my_rt.resource-type.yaml")).toBe("resource-type");
  });

  test("detects trigger types", () => {
    expect(getTypeStrFromPath("f/test/my_trig.http_trigger.yaml")).toBe("http_trigger");
    expect(getTypeStrFromPath("f/test/my_trig.websocket_trigger.yaml")).toBe("websocket_trigger");
    expect(getTypeStrFromPath("f/test/my_trig.kafka_trigger.yaml")).toBe("kafka_trigger");
  });

  test("detects folder metadata", () => {
    expect(getTypeStrFromPath("f/test/folder.meta.yaml")).toBe("folder");
  });

  test("detects user and group", () => {
    expect(getTypeStrFromPath("admin.user.yaml")).toBe("user");
    expect(getTypeStrFromPath("devs.group.yaml")).toBe("group");
  });

  test("throws for unknown type", () => {
    expect(() => getTypeStrFromPath("f/test/unknown.xyz.yaml")).toThrow();
  });
});

// =============================================================================
// validatePath
// =============================================================================

describe("validatePath", () => {
  test("accepts valid paths", () => {
    expect(validatePath("f/test/my_script")).toBe(true);
    expect(validatePath("u/admin/my_script")).toBe(true);
    expect(validatePath("g/all/my_script")).toBe(true);
  });

  test("rejects invalid paths", () => {
    expect(validatePath("invalid/path")).toBe(false);
    expect(validatePath("test/my_script")).toBe(false);
  });
});

// =============================================================================
// inferContentTypeFromFilePath
// =============================================================================

describe("inferContentTypeFromFilePath", () => {
  test("detects Python", () => {
    expect(inferContentTypeFromFilePath("script.py", undefined)).toBe("python3");
  });

  test("detects Go", () => {
    expect(inferContentTypeFromFilePath("script.go", undefined)).toBe("go");
  });

  test("detects Bash", () => {
    expect(inferContentTypeFromFilePath("script.sh", undefined)).toBe("bash");
  });

  test("detects PHP", () => {
    expect(inferContentTypeFromFilePath("script.php", undefined)).toBe("php");
  });

  test("detects Rust", () => {
    expect(inferContentTypeFromFilePath("script.rs", undefined)).toBe("rust");
  });

  test("detects PowerShell", () => {
    expect(inferContentTypeFromFilePath("script.ps1", undefined)).toBe("powershell");
  });

  test("detects GraphQL", () => {
    expect(inferContentTypeFromFilePath("query.gql", undefined)).toBe("graphql");
  });

  test("defaults .ts to bun", () => {
    expect(inferContentTypeFromFilePath("script.ts", undefined)).toBe("bun");
  });

  test("uses defaultTs for .ts files", () => {
    expect(inferContentTypeFromFilePath("script.ts", "deno")).toBe("deno");
    expect(inferContentTypeFromFilePath("script.ts", "bun")).toBe("bun");
  });

  test("explicit bun.ts and deno.ts override defaultTs", () => {
    expect(inferContentTypeFromFilePath("script.bun.ts", "deno")).toBe("bun");
    expect(inferContentTypeFromFilePath("script.deno.ts", "bun")).toBe("deno");
  });

  test("detects nativets with fetch.ts", () => {
    expect(inferContentTypeFromFilePath("script.fetch.ts", "bun")).toBe("nativets");
  });

  test("detects SQL variants", () => {
    expect(inferContentTypeFromFilePath("query.pg.sql", undefined)).toBe("postgresql");
    expect(inferContentTypeFromFilePath("query.my.sql", undefined)).toBe("mysql");
    expect(inferContentTypeFromFilePath("query.bq.sql", undefined)).toBe("bigquery");
    expect(inferContentTypeFromFilePath("query.ms.sql", undefined)).toBe("mssql");
    expect(inferContentTypeFromFilePath("query.sf.sql", undefined)).toBe("snowflake");
    expect(inferContentTypeFromFilePath("query.duckdb.sql", undefined)).toBe("duckdb");
    expect(inferContentTypeFromFilePath("query.odb.sql", undefined)).toBe("oracledb");
  });
});

// =============================================================================
// extractNativeTriggerInfo
// =============================================================================

describe("extractNativeTriggerInfo", () => {
  test("extracts info from valid flow trigger path", () => {
    const result = extractNativeTriggerInfo(
      "u/admin/script.flow.12345.nextcloud_native_trigger.json"
    );
    expect(result).not.toBeNull();
    expect(result!.scriptPath).toBe("u/admin/script");
    expect(result!.isFlow).toBe(true);
    expect(result!.externalId).toBe("12345");
    expect(result!.serviceName).toBe("nextcloud");
  });

  test("detects script (non-flow) triggers", () => {
    const result = extractNativeTriggerInfo(
      "f/test/handler.script.abc123.nextcloud_native_trigger.json"
    );
    expect(result).not.toBeNull();
    expect(result!.isFlow).toBe(false);
    expect(result!.scriptPath).toBe("f/test/handler");
  });

  test("returns null for non-native trigger paths", () => {
    expect(extractNativeTriggerInfo("f/test/my_var.variable.yaml")).toBeNull();
    expect(extractNativeTriggerInfo("f/test/trig.http_trigger.yaml")).toBeNull();
  });
});

// =============================================================================
// isSuperset
// =============================================================================

describe("isSuperset", () => {
  test("returns true when subset matches superset", () => {
    expect(isSuperset({ a: 1 }, { a: 1, b: 2 })).toBe(true);
  });

  test("returns true when objects are identical", () => {
    expect(isSuperset({ a: 1, b: 2 }, { a: 1, b: 2 })).toBe(true);
  });

  test("returns false when values differ", () => {
    expect(isSuperset({ a: 1 }, { a: 2 })).toBe(false);
  });

  test("handles nested objects", () => {
    expect(isSuperset({ a: { x: 1 } }, { a: { x: 1 }, b: 2 })).toBe(true);
    expect(isSuperset({ a: { x: 1 } }, { a: { x: 2 } })).toBe(false);
  });

  test("empty subset is always a superset match", () => {
    expect(isSuperset({}, { a: 1, b: 2 })).toBe(true);
  });
});

// =============================================================================
// filePathExtensionFromContentType
// =============================================================================

describe("filePathExtensionFromContentType", () => {
  test("returns .py for python3", () => {
    expect(filePathExtensionFromContentType("python3", undefined)).toBe(".py");
  });

  test("returns .fetch.ts for nativets", () => {
    expect(filePathExtensionFromContentType("nativets", undefined)).toBe(".fetch.ts");
  });

  test("returns .ts for bun when defaultTs is bun or undefined", () => {
    expect(filePathExtensionFromContentType("bun", "bun")).toBe(".ts");
    expect(filePathExtensionFromContentType("bun", undefined)).toBe(".ts");
  });

  test("returns .bun.ts for bun when defaultTs is deno", () => {
    expect(filePathExtensionFromContentType("bun", "deno")).toBe(".bun.ts");
  });

  test("returns .ts for deno when defaultTs is deno", () => {
    expect(filePathExtensionFromContentType("deno", "deno")).toBe(".ts");
  });

  test("returns .deno.ts for deno when defaultTs is bun or undefined", () => {
    expect(filePathExtensionFromContentType("deno", "bun")).toBe(".deno.ts");
    expect(filePathExtensionFromContentType("deno", undefined)).toBe(".deno.ts");
  });

  test("returns .go for go", () => {
    expect(filePathExtensionFromContentType("go", undefined)).toBe(".go");
  });

  test("returns .sh for bash", () => {
    expect(filePathExtensionFromContentType("bash", undefined)).toBe(".sh");
  });

  test("returns .ps1 for powershell", () => {
    expect(filePathExtensionFromContentType("powershell", undefined)).toBe(".ps1");
  });

  test("returns .gql for graphql", () => {
    expect(filePathExtensionFromContentType("graphql", undefined)).toBe(".gql");
  });

  test("returns .php for php", () => {
    expect(filePathExtensionFromContentType("php", undefined)).toBe(".php");
  });

  test("returns .rs for rust", () => {
    expect(filePathExtensionFromContentType("rust", undefined)).toBe(".rs");
  });

  test("returns .cs for csharp", () => {
    expect(filePathExtensionFromContentType("csharp", undefined)).toBe(".cs");
  });

  test("returns .nu for nu", () => {
    expect(filePathExtensionFromContentType("nu", undefined)).toBe(".nu");
  });

  test("returns .java for java", () => {
    expect(filePathExtensionFromContentType("java", undefined)).toBe(".java");
  });

  test("returns .rb for ruby", () => {
    expect(filePathExtensionFromContentType("ruby", undefined)).toBe(".rb");
  });

  test("returns .playbook.yml for ansible", () => {
    expect(filePathExtensionFromContentType("ansible", undefined)).toBe(".playbook.yml");
  });

  test("returns correct SQL extensions", () => {
    expect(filePathExtensionFromContentType("postgresql", undefined)).toBe(".pg.sql");
    expect(filePathExtensionFromContentType("mysql", undefined)).toBe(".my.sql");
    expect(filePathExtensionFromContentType("bigquery", undefined)).toBe(".bq.sql");
    expect(filePathExtensionFromContentType("duckdb", undefined)).toBe(".duckdb.sql");
    expect(filePathExtensionFromContentType("oracledb", undefined)).toBe(".odb.sql");
    expect(filePathExtensionFromContentType("snowflake", undefined)).toBe(".sf.sql");
    expect(filePathExtensionFromContentType("mssql", undefined)).toBe(".ms.sql");
  });

  test("throws for invalid language", () => {
    expect(() =>
      filePathExtensionFromContentType("invalid" as any, undefined)
    ).toThrow();
  });
});

// =============================================================================
// removeExtensionToPath
// =============================================================================

describe("removeExtensionToPath", () => {
  test("removes .ts extension", () => {
    expect(removeExtensionToPath("f/test/script.ts")).toBe("f/test/script");
  });

  test("removes .py extension", () => {
    expect(removeExtensionToPath("f/test/script.py")).toBe("f/test/script");
  });

  test("removes .go extension", () => {
    expect(removeExtensionToPath("f/test/script.go")).toBe("f/test/script");
  });

  test("removes .sh extension", () => {
    expect(removeExtensionToPath("f/test/script.sh")).toBe("f/test/script");
  });

  test("removes .pg.sql extension", () => {
    expect(removeExtensionToPath("f/test/query.pg.sql")).toBe("f/test/query");
  });

  test("removes .my.sql extension", () => {
    expect(removeExtensionToPath("f/test/query.my.sql")).toBe("f/test/query");
  });

  test("removes .duckdb.sql extension", () => {
    expect(removeExtensionToPath("f/test/query.duckdb.sql")).toBe("f/test/query");
  });

  test("removes .fetch.ts extension", () => {
    expect(removeExtensionToPath("f/test/script.fetch.ts")).toBe("f/test/script");
  });

  test("removes .bun.ts extension", () => {
    expect(removeExtensionToPath("f/test/script.bun.ts")).toBe("f/test/script");
  });

  test("removes .deno.ts extension", () => {
    expect(removeExtensionToPath("f/test/script.deno.ts")).toBe("f/test/script");
  });

  test("removes .gql extension", () => {
    expect(removeExtensionToPath("f/test/query.gql")).toBe("f/test/query");
  });

  test("removes .ps1 extension", () => {
    expect(removeExtensionToPath("f/test/script.ps1")).toBe("f/test/script");
  });

  test("removes .php extension", () => {
    expect(removeExtensionToPath("f/test/script.php")).toBe("f/test/script");
  });

  test("removes .rs extension", () => {
    expect(removeExtensionToPath("f/test/script.rs")).toBe("f/test/script");
  });

  test("removes .cs extension", () => {
    expect(removeExtensionToPath("f/test/script.cs")).toBe("f/test/script");
  });

  test("removes .nu extension", () => {
    expect(removeExtensionToPath("f/test/script.nu")).toBe("f/test/script");
  });

  test("removes .playbook.yml extension", () => {
    expect(removeExtensionToPath("f/test/play.playbook.yml")).toBe("f/test/play");
  });

  test("removes .java extension", () => {
    expect(removeExtensionToPath("f/test/Script.java")).toBe("f/test/Script");
  });

  test("removes .rb extension", () => {
    expect(removeExtensionToPath("f/test/script.rb")).toBe("f/test/script");
  });

  test("throws for unknown extension", () => {
    expect(() => removeExtensionToPath("f/test/file.xyz")).toThrow();
  });

  test("prioritizes longer extensions (fetch.ts over .ts)", () => {
    // fetch.ts should be recognized as nativets, not as bun .ts
    expect(removeExtensionToPath("f/test/api.fetch.ts")).toBe("f/test/api");
  });
});
