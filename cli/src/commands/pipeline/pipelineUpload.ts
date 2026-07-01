// Helpers for `wmill pipeline run --upload <script>[:<param>]=<file|s3://key>`:
// bind an object to a `data_upload` / `webhook` entry point so it (and its
// downstream) runs in the cascade instead of being skipped for want of input.
import { basename } from "node:path";

export type UploadBinding = { scriptTok: string; param?: string; source: string };

/**
 * Parse a `--upload` spec: `<script>[:<param>]=<file-or-s3-uri>`.
 * Split on the FIRST `=` so an `s3://…` source (whose `:` sits to the right of
 * it) stays unambiguous; an optional `:<param>` on the left names the target
 * S3Object argument when the script declares more than one.
 */
export function parseUploadBinding(spec: string): UploadBinding {
  const eq = spec.indexOf("=");
  const left = eq < 0 ? "" : spec.slice(0, eq).trim();
  const source = eq < 0 ? "" : spec.slice(eq + 1).trim();
  if (!left || !source) {
    throw new Error(`--upload '${spec}' must be <script>[:<param>]=<file-or-s3-uri>`);
  }
  const colon = left.indexOf(":");
  if (colon < 0) return { scriptTok: left, source };
  const scriptTok = left.slice(0, colon).trim();
  const param = left.slice(colon + 1).trim();
  if (!scriptTok || !param) {
    throw new Error(`--upload '${spec}' must be <script>[:<param>]=<file-or-s3-uri>`);
  }
  return { scriptTok, param, source };
}

/** Names of a schema's S3Object properties (`format: resource-s3_object`). */
export function s3ObjectParams(schema: any): string[] {
  const props = schema?.properties ?? {};
  return Object.keys(props).filter((k) => props[k]?.format === "resource-s3_object");
}

/**
 * Deterministic dev object key for a local sample bound to a script's param.
 * Scoped by script path + param so two different sources that share a basename
 * (across scripts or params) resolve to distinct keys instead of clobbering.
 */
export function devUploadKey(scriptPath: string, param: string, source: string): string {
  return `wmilldev/pipeline/${scriptPath}/${param}/${basename(source)}`;
}

/** The S3Object run-arg (`{ <param>: { s3: <key> } }`). */
export function s3Arg(param: string, key: string): Record<string, { s3: string }> {
  return { [param]: { s3: key } };
}
