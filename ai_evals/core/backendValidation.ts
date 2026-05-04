import type { EvalMode } from "./types";
import {
  parsePositiveInteger,
  resolveWindmillBackendSettings,
} from "./windmillBackendSettings";

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

export function parseBackendValidationMode(
  value?: string | null,
): BackendValidationMode {
  const normalized = value?.trim().toLowerCase();

  if (
    !normalized ||
    normalized === "off" ||
    normalized === "false" ||
    normalized === "0"
  ) {
    return "off";
  }

  if (normalized === "preview" || normalized === "true" || normalized === "1") {
    return "preview";
  }

  throw new Error(
    `Unsupported backend validation mode: ${value}. Use one of: ${BACKEND_VALIDATION_MODES.join(", ")}`,
  );
}

export function resolveBackendValidationSettings(input: {
  evalMode: EvalMode;
  requestedMode?: string | null;
}): BackendValidationSettings {
  const mode = parseBackendValidationMode(
    input.requestedMode ?? process.env.WMILL_AI_EVAL_BACKEND_VALIDATION,
  );

  if (
    mode !== "off" &&
    input.evalMode !== "flow" &&
    input.evalMode !== "script"
  ) {
    throw new Error(
      `Backend validation mode "${mode}" is only supported for flow and script evals`,
    );
  }

  return {
    mode,
    ...resolveWindmillBackendSettings(),
    pollIntervalMs: parsePositiveInteger(
      process.env.WMILL_AI_EVAL_BACKEND_POLL_INTERVAL_MS,
      2000,
    ),
    maxWaitMs: parsePositiveInteger(
      process.env.WMILL_AI_EVAL_BACKEND_MAX_WAIT_MS,
      120000,
    ),
  };
}
