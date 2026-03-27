import { parse as yamlParse } from "yaml";
import type { ParseOptions, DocumentOptions, SchemaOptions, ToJSOptions, ScalarTag } from "yaml";
import { readFile } from "node:fs/promises";

// Custom YAML tags that resolve `!inline value` and `!inline_fileset value`
// back to their string-prefix form ("!inline value").
// Without these, the yaml parser strips the tag and returns just the scalar,
// breaking the string-prefix-based !inline detection used throughout the CLI.
const inlineTag: ScalarTag = {
  tag: "!inline",
  resolve(value: string) {
    return "!inline " + value;
  },
};

const inlineFilesetTag: ScalarTag = {
  tag: "!inline_fileset",
  resolve(value: string) {
    return "!inline_fileset " + value;
  },
};

const WINDMILL_CUSTOM_TAGS: ScalarTag[] = [inlineTag, inlineFilesetTag];

type YamlParseOptions = ParseOptions & DocumentOptions & SchemaOptions & ToJSOptions;

export async function yamlParseFile(path: string, options: YamlParseOptions = {}) {
  try {
    return yamlParse(await readFile(path, "utf-8"), {
      ...options,
      customTags: WINDMILL_CUSTOM_TAGS,
    });
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}

export function yamlParseContent(
  path: string,
  content: string,
  options: YamlParseOptions = {},
) {
  try {
    return yamlParse(content, {
      ...options,
      customTags: WINDMILL_CUSTOM_TAGS,
    });
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}
