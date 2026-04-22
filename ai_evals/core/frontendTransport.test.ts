import { afterEach, describe, expect, it } from "bun:test";
import {
  parseFrontendEvalTransport,
  resolveFrontendEvalTransportSettings,
} from "./frontendTransport";

const ORIGINAL_ENV = {
  WMILL_AI_EVAL_BACKEND_URL: process.env.WMILL_AI_EVAL_BACKEND_URL,
};

afterEach(() => {
  if (ORIGINAL_ENV.WMILL_AI_EVAL_BACKEND_URL === undefined) {
    delete process.env.WMILL_AI_EVAL_BACKEND_URL;
  } else {
    process.env.WMILL_AI_EVAL_BACKEND_URL =
      ORIGINAL_ENV.WMILL_AI_EVAL_BACKEND_URL;
  }
});

describe("parseFrontendEvalTransport", () => {
  it("defaults to direct when unset", () => {
    expect(parseFrontendEvalTransport(undefined)).toBe("direct");
  });

  it("accepts proxy explicitly", () => {
    expect(parseFrontendEvalTransport("proxy")).toBe("proxy");
  });

  it("rejects unsupported values", () => {
    expect(() => parseFrontendEvalTransport("worker")).toThrow(
      "Unsupported frontend eval transport: worker",
    );
  });
});

describe("resolveFrontendEvalTransportSettings", () => {
  it("includes backend settings for proxy transport", () => {
    process.env.WMILL_AI_EVAL_BACKEND_URL = "http://127.0.0.1:8000/";

    expect(
      resolveFrontendEvalTransportSettings({
        evalMode: "app",
        requestedTransport: "proxy",
      }),
    ).toMatchObject({
      transport: "proxy",
      backend: {
        baseUrl: "http://127.0.0.1:8000",
      },
    });
  });

  it("keeps direct transport for cli runs", () => {
    expect(
      resolveFrontendEvalTransportSettings({
        evalMode: "cli",
        requestedTransport: "direct",
      }),
    ).toEqual({
      transport: "direct",
      backend: undefined,
    });
  });
});
