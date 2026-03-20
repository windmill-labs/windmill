/**
 * Relative Imports Utilities for CLI
 *
 * Provides functions to parse relative imports from TypeScript/Python scripts using WASM.
 */

import { ScriptLanguage } from "./script_common.ts";
import { loadParser } from "./metadata.ts";
import * as log from "../core/log.ts";

/**
 * Extract relative imports from script content based on language.
 * Returns resolved absolute Windmill paths (e.g., "f/folder/helper").
 */
export async function extractRelativeImports(
  code: string,
  scriptPath: string,
  language: ScriptLanguage
): Promise<string[]> {
  try {
    switch (language) {
      case "bun":
      case "nativets":
      case "deno": {
        const { parse_ts_relative_imports } = await loadParser("windmill-parser-wasm-ts");
        return parse_ts_relative_imports(code, scriptPath);
      }
      case "python3": {
        const { parse_py_relative_imports } = await loadParser("windmill-parser-wasm-py-imports");
        return parse_py_relative_imports(code, scriptPath);
      }
      default:
        return [];
    }
  } catch (e) {
    log.warn(`Failed to parse relative imports for ${scriptPath}: ${e}. Dependency tracking for relative imports will be disabled.`);
    return [];
  }
}
