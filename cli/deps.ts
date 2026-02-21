// cliffy
export { Command } from "@cliffy/command";
export { Table } from "@cliffy/table";
export { colors } from "@cliffy/ansi/colors";
export { Secret } from "@cliffy/prompt/secret";
export { Select } from "@cliffy/prompt/select";
export { Confirm } from "@cliffy/prompt/confirm";
export { Input } from "@cliffy/prompt/input";
export { UpgradeCommand } from "@cliffy/command/upgrade";
export { NpmProvider } from "@cliffy/command/upgrade/provider/npm";
export { Provider } from "@cliffy/command/upgrade";

export { CompletionsCommand } from "@cliffy/command/completions";
// std
export { ensureDir } from "@std/fs";
export { SEPARATOR as SEP } from "@std/path";
export * as path from "@std/path";
export { encodeHex } from "@std/encoding";
export { writeAllSync } from "@std/io/write-all";
export { copy } from "@std/io/copy";
export { readAll } from "@std/io/read-all";

export * as log from "@std/log";
export { stringify as yamlStringify } from "@std/yaml";

import { parse as yamlParse, ParseOptions } from "@std/yaml";
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

// other

export * as Diff from "diff";
export { minimatch } from "minimatch";
export { default as JSZip } from "jszip";

export * as express from "express";
export * as http from "node:http";
export { WebSocket, WebSocketServer } from "ws";
export * as getPort from "get-port";
export * as open from "open";
export * as esMain from "es-main";
export * as windmillUtils from "@windmill-labs/shared-utils";

import { OpenAPI } from "./gen/index.ts";

export function setClient(token?: string, baseUrl?: string) {
  if (baseUrl === undefined) {
    baseUrl = process.env["BASE_INTERNAL_URL"] ??
      process.env["BASE_URL"] ??
      "http://localhost:8000";
  }
  if (token === undefined) {
    token = process.env["WM_TOKEN"] ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}
