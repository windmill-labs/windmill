import { existsSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { stringify as yamlStringify } from "yaml";

import { yamlParseFile } from "../../utils/yaml.ts";
import { yamlOptions } from "../sync/sync.ts";
import {
  SyncOptions,
  getWmillYamlPath,
  getWorkspaceNames,
  getEffectiveWorkspaceId,
  WorkspaceEntryConfig,
} from "../../core/conf.ts";
import { GlobalOptions } from "../../types.ts";
import { tryResolveBranchWorkspace } from "../../core/context.ts";
import { setClient } from "../../core/client.ts";
import { ProtectionRulesFile } from "./types.ts";

export const PROTECTION_RULES_FILENAME = "protection-rules.yaml";

// protection-rules.yaml lives next to wmill.yaml. wmill.yaml is required: it is
// the single source of truth for which workspaces exist and how to reach them.
export function getProtectionRulesPath(): string | null {
  const wmillPath = getWmillYamlPath();
  if (!wmillPath) return null;
  return join(dirname(wmillPath), PROTECTION_RULES_FILENAME);
}

export async function readProtectionRulesFile(
  path: string,
): Promise<ProtectionRulesFile> {
  if (!existsSync(path)) return {};
  const parsed = (await yamlParseFile(path)) as ProtectionRulesFile | null;
  return parsed ?? {};
}

export async function writeProtectionRulesFile(
  path: string,
  data: ProtectionRulesFile,
): Promise<void> {
  // Deterministic key order so diffs/commits stay stable.
  const sorted: ProtectionRulesFile = {};
  for (const k of Object.keys(data).sort()) sorted[k] = data[k];
  await writeFile(path, yamlStringify(sorted, yamlOptions), "utf-8");
}

// Maps a protection-rules.yaml workspace key to its backend workspace id via
// wmill.yaml's `workspaces` block. A key with no matching entry is rejected —
// without it we don't know which backend to talk to.
export class WorkspaceResolver {
  private constructor(
    private readonly workspaces: Record<string, WorkspaceEntryConfig>,
  ) {}

  static fromConfig(config: SyncOptions): WorkspaceResolver {
    const ws = (config.workspaces ?? {}) as Record<
      string,
      WorkspaceEntryConfig
    >;
    return new WorkspaceResolver(ws);
  }

  /** Workspace keys declared in wmill.yaml (excludes reserved keys). */
  knownNames(): string[] {
    return getWorkspaceNames(this.workspaces as any);
  }

  has(name: string): boolean {
    return this.knownNames().includes(name);
  }

  /** Backend workspace id (path param) for a key, or throw if unknown. */
  backendId(name: string): string {
    if (!this.has(name)) {
      throw new Error(
        `Workspace '${name}' is not defined in wmill.yaml 'workspaces'. ` +
          `Add it there (its keys must match protection-rules.yaml).`,
      );
    }
    return getEffectiveWorkspaceId(name, this.workspaces[name]);
  }
}

// Point the API client at the backend for a single wmill.yaml workspace key,
// then return the backend workspace id to use as the path param. The backend
// id always comes from the wmill.yaml mapping (the feature's invariant);
// credentials are resolved with the same precedence as every other command:
//
//   1. explicit --base-url + --token  -> used as-is (stateless CI; no profile
//      or wmill.yaml baseUrl required)
//   2. otherwise, the stored profile matching wmill.yaml workspaces.<ws>
//      (its baseUrl + token), with an explicit --token overriding the
//      stored token
//
// Throws a clean error if the key is unknown or nothing resolves it — callers
// decide whether to skip (--all) or fail (named arg).
export async function configureClientForWorkspace(
  opts: GlobalOptions,
  ws: string,
  resolver: WorkspaceResolver,
): Promise<string> {
  const wsId = resolver.backendId(ws); // throws if not in wmill.yaml

  // 1. Explicit credentials — honor them directly, like other commands do.
  if (opts.baseUrl) {
    if (!opts.token) {
      throw new Error(
        "When --base-url is set, --token is required for protection-rules.",
      );
    }
    setClient(opts.token, opts.baseUrl.replace(/\/+$/, ""));
    return wsId;
  }

  // 2. Stored-profile resolution. Fresh opts so resolveWorkspace's per-call
  // cache can't bleed across keys.
  const resolved = await tryResolveBranchWorkspace({ ...opts }, ws);
  if (!resolved) {
    throw new Error(
      `Could not resolve credentials for workspace '${ws}'. Either pass ` +
        `--base-url and --token, or ensure wmill.yaml workspaces.${ws} has a ` +
        `baseUrl and you've run 'wmill workspace add' for it.`,
    );
  }
  // An explicit --token overrides the stored profile's token.
  setClient(opts.token ?? resolved.token, resolved.remote.replace(/\/+$/, ""));
  return wsId;
}
