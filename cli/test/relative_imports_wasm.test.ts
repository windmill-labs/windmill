/**
 * Tests that WASM parsers correctly export and resolve relative imports.
 * Catches the bug where parse_ts_relative_imports was missing from the WASM build,
 * causing all TS relative imports to silently return [].
 */

import { expect, test, describe } from "bun:test";
import { loadParser } from "../src/utils/metadata.ts";
import { extractRelativeImports } from "../src/utils/relative_imports.ts";

describe("WASM TS parser exports parse_ts_relative_imports", () => {
  test("parse_ts_relative_imports function exists in WASM module", async () => {
    const mod = await loadParser("windmill-parser-wasm-ts");
    expect(typeof mod.parse_ts_relative_imports).toBe("function");
  });

  test("resolves dot-relative import", async () => {
    const code = `import { helper } from "./helper";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper"]);
  });

  test("resolves double-dot-relative import", async () => {
    const code = `import { utils } from "../utils/helper";\nexport async function main() { return utils(); }`;
    const result = await extractRelativeImports(code, "f/folder/sub/script", "bun");
    expect(result).toEqual(["f/folder/utils/helper"]);
  });

  test("resolves absolute windmill import", async () => {
    const code = `import { shared } from "/f/shared/utils";\nexport async function main() { return shared(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/shared/utils"]);
  });

  test("ignores external package imports", async () => {
    const code = `import lodash from "lodash";\nimport axios from "axios";\nexport async function main() { return lodash.map([]); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual([]);
  });

  test("strips .ts extension from imports", async () => {
    const code = `import { helper } from "./helper.ts";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper"]);
  });

  test("resolves mixed relative and external imports", async () => {
    const code = `import { helper } from "./helper";\nimport { utils } from "../utils";\nimport lodash from "lodash";\nexport async function main() { return helper(); }`;
    const result = await extractRelativeImports(code, "f/folder/script", "bun");
    expect(result).toEqual(["f/folder/helper", "f/utils"]);
  });

  test("works with named imports", async () => {
    const code = `import { slugify, capitalize } from "./string_helpers";\nexport async function main() { return slugify("test"); }`;
    const result = await extractRelativeImports(code, "f/utils/http_client", "bun");
    expect(result).toEqual(["f/utils/string_helpers"]);
  });
});

describe("WASM Python parser exports parse_py_relative_imports", () => {
  test("parse_py_relative_imports function exists in WASM module", async () => {
    const mod = await loadParser("windmill-parser-wasm-py-imports");
    expect(typeof mod.parse_py_relative_imports).toBe("function");
  });

  test("resolves python relative import", async () => {
    const code = `from f.utils.formatter import format_stats\ndef main(values: list):\n    return format_stats(values)`;
    const result = await extractRelativeImports(code, "f/data/process", "python3");
    expect(result).toEqual(["f/utils/formatter"]);
  });
});
