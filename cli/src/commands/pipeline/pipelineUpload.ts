// Helpers for `wmill pipeline run` per-script bindings:
// `--upload <script>[:<param>]=<file|s3://key>` binds an object to a
// `data_upload` / `webhook` entry point so it (and its downstream) runs in the
// cascade instead of being skipped for want of input;
// `--arg <script>:<param>=<value>` passes a plain (non-S3Object) run arg.
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

export type ArgBinding = { scriptTok: string; param: string; value: unknown };

/**
 * Parse an `--arg` spec: `<script>:<param>=<value>`. Split on the FIRST `=`
 * (values may contain `=`); the value is JSON when it parses as JSON, else the
 * raw string — so `stats:limit=10` is a number and
 * `daily_report:partition=2026-07-02` a string. `:<param>` is required: unlike
 * `--upload` there is no single-S3Object convention to infer it from.
 */
export function parseArgBinding(spec: string): ArgBinding {
  const eq = spec.indexOf("=");
  const left = eq < 0 ? "" : spec.slice(0, eq).trim();
  const raw = eq < 0 ? "" : spec.slice(eq + 1).trim();
  const colon = left.indexOf(":");
  const scriptTok = colon < 0 ? "" : left.slice(0, colon).trim();
  const param = colon < 0 ? "" : left.slice(colon + 1).trim();
  if (!scriptTok || !param) {
    throw new Error(`--arg '${spec}' must be <script>:<param>=<json-or-string>`);
  }
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    value = raw;
  }
  return { scriptTok, param, value };
}

/**
 * Names of a schema's S3Object properties. The resource name in `format`
 * varies by language parser — TS/python emit `resource-s3_object`, the SQL
 * dialects emit `resource-S3Object` — so match it case/underscore-insensitively
 * the way the frontend's ArgInput does.
 */
export function s3ObjectParams(schema: any): string[] {
  const props = schema?.properties ?? {};
  const isS3Object = (format: unknown): boolean =>
    typeof format === "string" &&
    format.startsWith("resource-") &&
    format.slice("resource-".length).replaceAll("_", "").toLowerCase() === "s3object";
  return Object.keys(props).filter((k) => isS3Object(props[k]?.format));
}

/**
 * Deterministic dev object key for a local sample bound to a script's param.
 * Scoped by script path + param so two different sources that share a basename
 * (across scripts or params) resolve to distinct keys instead of clobbering.
 */
export function devUploadKey(scriptPath: string, param: string, source: string): string {
  return `wmilldev/pipeline/${scriptPath}/${param}/${basename(source)}`;
}

export type S3Object = { s3: string; storage?: string };

/**
 * Parse an `s3://<storage>/<key>` source into an S3Object, matching Windmill's
 * canonical `parseS3Object` (frontend `utils.ts`): the authority is the named
 * storage — empty ⇒ default — and the rest is the object key. So
 * `s3://secondary/k.csv` → `{ s3: "k.csv", storage: "secondary" }`, and the
 * default-storage form is `s3:///key` (empty authority), including nested keys
 * (`s3:///raw/2026/events.csv`). A source with no `/` after the scheme has no
 * authority, so the whole remainder is the key in the default store.
 */
export function parseS3Uri(source: string): S3Object {
  const m = source.match(/^s3:\/\/([^/]*)\/(.*)$/);
  if (!m) return { s3: source.slice("s3://".length) };
  const storage = m[1] || undefined;
  return storage ? { s3: m[2], storage } : { s3: m[2] };
}

/** The S3Object run-arg (`{ <param>: <S3Object> }`). */
export function s3Arg(param: string, obj: S3Object): Record<string, S3Object> {
  return { [param]: obj };
}
