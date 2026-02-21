import { parse as yamlParse, type ParseOptions } from "@std/yaml";
import { readFile } from "node:fs/promises";

export async function yamlParseFile(path: string, options: ParseOptions = {}) {
  try {
    return yamlParse(await readFile(path, "utf-8"), options);
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}

export function yamlParseContent(
  path: string,
  content: string,
  options: ParseOptions = {},
) {
  try {
    return yamlParse(content, options);
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}
