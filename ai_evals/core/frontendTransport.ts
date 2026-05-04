import type { EvalMode } from "./types";
import type { WindmillBackendSettings } from "./windmillBackendSettings";
import { resolveWindmillBackendSettings } from "./windmillBackendSettings";

export const FRONTEND_EVAL_TRANSPORTS = ["direct", "proxy"] as const;

export type FrontendEvalTransport = (typeof FRONTEND_EVAL_TRANSPORTS)[number];

export interface FrontendEvalTransportSettings {
  transport: FrontendEvalTransport;
  backend?: WindmillBackendSettings;
}

export function parseFrontendEvalTransport(
  value?: string | null,
): FrontendEvalTransport {
  const normalized = value?.trim().toLowerCase();

  if (!normalized || normalized === "direct") {
    return "direct";
  }

  if (normalized === "proxy") {
    return "proxy";
  }

  throw new Error(
    `Unsupported frontend eval transport: ${value}. Use one of: ${FRONTEND_EVAL_TRANSPORTS.join(", ")}`,
  );
}

export function resolveFrontendEvalTransportSettings(input: {
  evalMode: EvalMode;
  requestedTransport?: string | null;
}): FrontendEvalTransportSettings {
  const transport = parseFrontendEvalTransport(input.requestedTransport);

  if (transport === "proxy" && input.evalMode === "cli") {
    throw new Error(
      'Frontend eval transport "proxy" is only supported for flow, script, and app evals',
    );
  }

  return {
    transport,
    backend:
      transport === "proxy" ? resolveWindmillBackendSettings() : undefined,
  };
}
