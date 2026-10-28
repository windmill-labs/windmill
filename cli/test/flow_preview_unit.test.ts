/**
 * `wmill flow preview` argument resolution and `--step` script loading.
 */
import { expect, test, describe, beforeEach, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { mockServices } from "./mock_services.ts";

const calls: { fn: string; args: any }[] = [];
mockServices({
  getHubScriptByPath: async (args: any) => {
    calls.push({ fn: "getHubScriptByPath", args });
    return {
      content: "export async function main() { return 'hub' }",
      language: "deno",
      lockfile: "hub-lock",
    };
  },
  getScriptByPath: async (args: any) => {
    calls.push({ fn: "getScriptByPath", args });
    return { content: "workspace code", language: "bun", lock: "ws-lock" };
  },
  getScriptByHash: async (args: any) => {
    calls.push({ fn: "getScriptByHash", args });
    return { content: "hashed code", language: "python3" };
  },
});

const { resolveFlowFolderArg, loadStepScript } = await import(
  "../src/commands/flow/flow.ts"
);

const SEP = path.sep;

describe("resolveFlowFolderArg", () => {
  let root: string;
  let previousCwd: string;

  beforeEach(() => {
    previousCwd = process.cwd();
    root = fs.realpathSync(
      fs.mkdtempSync(path.join(os.tmpdir(), "wmill_flow_preview_")),
    );
    fs.mkdirSync(path.join(root, "f", "t", "dotted.flow"), { recursive: true });
    fs.mkdirSync(path.join(root, "f", "t", "nondotted__flow"), { recursive: true });
    process.chdir(root);
  });

  afterEach(() => {
    process.chdir(previousCwd);
    fs.rmSync(root, { recursive: true, force: true });
  });

  test("a Windmill path resolves to its local .flow folder", () => {
    expect(resolveFlowFolderArg(`f${SEP}t${SEP}dotted`)).toBe(
      `f${SEP}t${SEP}dotted.flow${SEP}`,
    );
  });

  test("a Windmill path resolves to a __flow folder when that is the one on disk", () => {
    expect(resolveFlowFolderArg(`f${SEP}t${SEP}nondotted`)).toBe(
      `f${SEP}t${SEP}nondotted__flow${SEP}`,
    );
  });

  test("the folder and its flow.yaml keep resolving to the folder", () => {
    const folder = `f${SEP}t${SEP}dotted.flow${SEP}`;
    expect(resolveFlowFolderArg(`f${SEP}t${SEP}dotted.flow`)).toBe(folder);
    expect(resolveFlowFolderArg(folder)).toBe(folder);
    expect(resolveFlowFolderArg(`${folder}flow.yaml`)).toBe(folder);
    expect(resolveFlowFolderArg("flow.yaml")).toBe(`.${SEP}`);
  });

  test("a Windmill path with no local flow folder is refused, naming what was looked for", () => {
    expect(() => resolveFlowFolderArg(`f${SEP}t${SEP}missing`)).toThrow(
      `found none of f${SEP}t${SEP}missing.flow, f${SEP}t${SEP}missing__flow`,
    );
  });
});

describe("loadStepScript", () => {
  beforeEach(() => {
    calls.length = 0;
  });

  test("a hub path is fetched from the hub, with the hub's lockfile", async () => {
    const script = await loadStepScript("ws", {
      path: "hub/29624/slack/send_message_to_channel",
    });
    expect(calls).toEqual([
      {
        fn: "getHubScriptByPath",
        args: { path: "hub/29624/slack/send_message_to_channel" },
      },
    ]);
    expect(script).toEqual({
      content: "export async function main() { return 'hub' }",
      language: "deno",
      lock: "hub-lock",
    });
  });

  test("a workspace path is fetched from the workspace", async () => {
    const script = await loadStepScript("ws", { path: "f/t/helper" });
    expect(calls).toEqual([
      { fn: "getScriptByPath", args: { workspace: "ws", path: "f/t/helper" } },
    ]);
    expect(script).toEqual({ content: "workspace code", language: "bun" });
  });

  test("a pinned hash is fetched by hash", async () => {
    const script = await loadStepScript("ws", { path: "f/t/helper", hash: "abc" });
    expect(calls).toEqual([
      { fn: "getScriptByHash", args: { workspace: "ws", hash: "abc" } },
    ]);
    expect(script).toEqual({ content: "hashed code", language: "python3" });
  });
});
