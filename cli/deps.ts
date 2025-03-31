// cliffy
export { Command } from "jsr:@windmill-labs/cliffy-command@1.0.0-rc.5";
export { Table } from "jsr:@windmill-labs/cliffy-table@1.0.0-rc.5";
export { colors } from "jsr:@windmill-labs/cliffy-ansi@1.0.0-rc.5/colors";
export { Secret } from "jsr:@windmill-labs/cliffy-prompt@1.0.0-rc.6/secret";
export { Select } from "jsr:@windmill-labs/cliffy-prompt@1.0.0-rc.6/select";
export { Confirm } from "jsr:@windmill-labs/cliffy-prompt@1.0.0-rc.6/confirm";
export { Input } from "jsr:@windmill-labs/cliffy-prompt@1.0.0-rc.6/input";
export { UpgradeCommand } from "jsr:@windmill-labs/cliffy-command@1.0.0-rc.5/upgrade";
export { NpmProvider } from "jsr:@windmill-labs/cliffy-command@1.0.0-rc.5/upgrade/provider/npm";
export { Provider } from "jsr:@windmill-labs/cliffy-command@1.0.0-rc.5/upgrade";

export { CompletionsCommand } from "jsr:@windmill-labs/cliffy-command@1.0.0-rc.5/completions";
// std
export { ensureDir } from "jsr:@std/fs";
export { SEPARATOR as SEP } from "jsr:@std/path";
export * as path from "jsr:@std/path";
export { encodeHex } from "jsr:@std/encoding@1.0.4";
export { writeAllSync } from "jsr:@std/io/write-all";
export { copy } from "jsr:@std/io/copy";
export { readAll } from "jsr:@std/io/read-all";

export * as log from "jsr:@std/log";
export { stringify as yamlStringify } from "jsr:@std/yaml";

import { parse as yamlParse, ParseOptions } from "jsr:@std/yaml";

export async function yamlParseFile(path: string, options: ParseOptions = {}) {
  try {
    return yamlParse(await Deno.readTextFile(path), options);
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}

export function yamlParseContent(
  path: string,
  content: string,
  options: ParseOptions = {}
) {
  try {
    return yamlParse(content, options);
  } catch (e) {
    throw new Error(`Error parsing yaml ${path}`, { cause: e });
  }
}

// other

export * as Diff from "npm:diff";
export { minimatch } from "npm:minimatch";
export { default as JSZip } from "npm:jszip@3.7.1";

export * as express from "npm:express";
export * as http from "node:http";
export { WebSocketServer, WebSocket } from "npm:ws";
export * as getPort from "npm:get-port@7.1.0";
export * as open from "npm:open";
export * as esMain from "npm:es-main";

import { OpenAPI } from "./gen/index.ts";

export function setClient(token?: string, baseUrl?: string) {
  if (baseUrl === undefined) {
    baseUrl =
      getEnv("BASE_INTERNAL_URL") ??
      getEnv("BASE_URL") ??
      "http://localhost:8000";
  }
  if (token === undefined) {
    token = getEnv("WM_TOKEN") ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}

const getEnv = (key: string) => {
  return Deno.env.get(key);
};
