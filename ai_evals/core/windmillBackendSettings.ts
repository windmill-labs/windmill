export interface WindmillBackendSettings {
  baseUrl: string;
  email: string;
  password: string;
  keepWorkspaces: boolean;
  workspaceOverride?: string;
  workspacePrefix: string;
}

export function resolveWindmillBackendSettings(): WindmillBackendSettings {
  return {
    baseUrl: normalizeBaseUrl(
      process.env.WMILL_AI_EVAL_BACKEND_URL ??
        process.env.WINDMILL_URL ??
        process.env.WINDMILL_BASE_URL ??
        process.env.REMOTE ??
        "http://127.0.0.1:8000",
    ),
    email: process.env.WMILL_AI_EVAL_BACKEND_EMAIL ?? "admin@windmill.dev",
    password: process.env.WMILL_AI_EVAL_BACKEND_PASSWORD ?? "changeme",
    keepWorkspaces: isTruthy(process.env.WMILL_AI_EVAL_KEEP_WORKSPACES),
    workspaceOverride: sanitizeOptionalWorkspaceId(
      process.env.WMILL_AI_EVAL_BACKEND_WORKSPACE,
    ),
    workspacePrefix: sanitizeWorkspacePrefix(
      process.env.WMILL_AI_EVAL_WORKSPACE_PREFIX ?? "ai-evals",
    ),
  };
}

export function parsePositiveInteger(
  value: string | undefined,
  fallback: number,
): number {
  if (!value) {
    return fallback;
  }
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
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

function sanitizeOptionalWorkspaceId(
  value: string | undefined,
): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function isTruthy(value: string | undefined): boolean {
  if (!value) {
    return false;
  }
  return ["1", "true", "yes", "on"].includes(value.trim().toLowerCase());
}
