import type { EvalMode } from "./types";

export const BACKEND_VALIDATION_MODES = ["off", "preview"] as const;

export type BackendValidationMode = (typeof BACKEND_VALIDATION_MODES)[number];

export interface BackendValidationSettings {
  mode: BackendValidationMode;
  baseUrl: string;
  email: string;
  password: string;
  keepWorkspaces: boolean;
  workspaceOverride?: string;
  workspacePrefix: string;
  pollIntervalMs: number;
  maxWaitMs: number;
}

export function parseBackendValidationMode(value?: string | null): BackendValidationMode {
  const normalized = value?.trim().toLowerCase();

  if (!normalized || normalized === "off" || normalized === "false" || normalized === "0") {
    return "off";
  }

  if (normalized === "preview" || normalized === "true" || normalized === "1") {
    return "preview";
  }

  throw new Error(
    `Unsupported backend validation mode: ${value}. Use one of: ${BACKEND_VALIDATION_MODES.join(", ")}`
  );
}

export function resolveBackendValidationSettings(input: {
  evalMode: EvalMode;
  requestedMode?: string | null;
}): BackendValidationSettings {
  const mode = parseBackendValidationMode(
    input.requestedMode ?? process.env.WMILL_AI_EVAL_BACKEND_VALIDATION
  );

  if (mode !== "off" && input.evalMode !== "flow" && input.evalMode !== "script") {
    throw new Error(
      `Backend validation mode "${mode}" is only supported for flow and script evals`
    );
  }

  return {
    mode,
    baseUrl: normalizeBaseUrl(
      process.env.WMILL_AI_EVAL_BACKEND_URL ??
        process.env.WINDMILL_URL ??
        process.env.WINDMILL_BASE_URL ??
        process.env.REMOTE ??
        "http://127.0.0.1:8000"
    ),
    email: process.env.WMILL_AI_EVAL_BACKEND_EMAIL ?? "admin@windmill.dev",
    password: process.env.WMILL_AI_EVAL_BACKEND_PASSWORD ?? "changeme",
    keepWorkspaces: isTruthy(process.env.WMILL_AI_EVAL_KEEP_WORKSPACES),
    workspaceOverride: sanitizeOptionalWorkspaceId(process.env.WMILL_AI_EVAL_BACKEND_WORKSPACE),
    workspacePrefix: sanitizeWorkspacePrefix(
      process.env.WMILL_AI_EVAL_WORKSPACE_PREFIX ?? "ai-evals"
    ),
    pollIntervalMs: parsePositiveInteger(
      process.env.WMILL_AI_EVAL_BACKEND_POLL_INTERVAL_MS,
      2000
    ),
    maxWaitMs: parsePositiveInteger(process.env.WMILL_AI_EVAL_BACKEND_MAX_WAIT_MS, 120000),
  };
}

function normalizeBaseUrl(value: string): string {
  return value.replace(/\/+$/, "");
}

function sanitizeWorkspacePrefix(value: string): string {
  const sanitized = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return sanitized.length > 0 ? sanitized : "ai-evals";
}

function sanitizeOptionalWorkspaceId(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function isTruthy(value: string | undefined): boolean {
  if (!value) {
    return false;
  }
  return ["1", "true", "yes", "on"].includes(value.trim().toLowerCase());
}

function parsePositiveInteger(value: string | undefined, fallback: number): number {
  if (!value) {
    return fallback;
  }
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}
