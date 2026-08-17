import { afterEach, describe, expect, it } from "bun:test";
import { resolveWindmillBackendSettings } from "./windmillBackendSettings";

const ENV_KEYS = [
  "WMILL_AI_EVAL_BACKEND_URL",
  "WINDMILL_URL",
  "WINDMILL_BASE_URL",
  "REMOTE",
  "WMILL_AI_EVAL_BACKEND_EMAIL",
  "WMILL_AI_EVAL_BACKEND_PASSWORD",
  "WMILL_AI_EVAL_BACKEND_WORKSPACE",
] as const;

const ORIGINAL_ENV = Object.fromEntries(
  ENV_KEYS.map((key) => [key, process.env[key]]),
) as Record<(typeof ENV_KEYS)[number], string | undefined>;

afterEach(() => {
  for (const key of ENV_KEYS) {
    const value = ORIGINAL_ENV[key];
    if (value === undefined) {
      delete process.env[key];
    } else {
      process.env[key] = value;
    }
  }
});

describe("resolveWindmillBackendSettings", () => {
  it("uses backend URL/auth defaults and the optional explicit workspace", () => {
    delete process.env.WMILL_AI_EVAL_BACKEND_URL;
    delete process.env.WINDMILL_URL;
    delete process.env.WINDMILL_BASE_URL;
    delete process.env.REMOTE;
    process.env.WMILL_AI_EVAL_BACKEND_WORKSPACE = "shared-evals";

    expect(resolveWindmillBackendSettings()).toEqual({
      baseUrl: "http://127.0.0.1:8000",
      email: "admin@windmill.dev",
      password: "changeme",
      workspaceOverride: "shared-evals",
    });
  });

  it("does not expose workspace retention knobs", () => {
    process.env.WMILL_AI_EVAL_BACKEND_URL = "http://backend.test/";

    const settings = resolveWindmillBackendSettings();

    expect(settings).toEqual({
      baseUrl: "http://backend.test",
      email: "admin@windmill.dev",
      password: "changeme",
      workspaceOverride: undefined,
    });
    expect(Object.keys(settings).sort()).toEqual([
      "baseUrl",
      "email",
      "password",
      "workspaceOverride",
    ]);
  });
});
