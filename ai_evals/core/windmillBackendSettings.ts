export interface WindmillBackendSettings {
  baseUrl: string;
  email: string;
  password: string;
  workspaceOverride?: string;
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
    workspaceOverride: sanitizeOptionalWorkspaceId(
      process.env.WMILL_AI_EVAL_BACKEND_WORKSPACE,
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

function sanitizeOptionalWorkspaceId(
  value: string | undefined,
): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
