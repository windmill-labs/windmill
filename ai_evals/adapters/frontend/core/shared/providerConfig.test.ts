import { describe, expect, it } from "bun:test";
import {
  buildProxyHeaders,
  buildProxyResourcePath,
  buildOpenAICompatibleClientOptions,
  resolveEvalModelProvider,
} from "./providerConfig";

describe("buildOpenAICompatibleClientOptions", () => {
  it("adds Gemini's OpenAI-compatible base URL and client header", () => {
    const options = buildOpenAICompatibleClientOptions(
      "googleai",
      "gemini-test-key",
    );

    expect(options).toMatchObject({
      apiKey: "gemini-test-key",
      baseURL: "https://generativelanguage.googleapis.com/v1beta/openai/",
      defaultHeaders: {
        "x-goog-api-client": "windmill-ai-evals/1.0",
      },
    });
  });

  it("keeps the default OpenAI-compatible config for OpenAI", () => {
    expect(
      buildOpenAICompatibleClientOptions("openai", "openai-test-key"),
    ).toEqual({
      apiKey: "openai-test-key",
    });
  });
});

describe("proxy helpers", () => {
  it("builds provider-scoped proxy resource paths", () => {
    expect(buildProxyResourcePath("googleai")).toBe("f/evals/ai/googleai");
    expect(buildProxyResourcePath("anthropic")).toBe("f/evals/ai/anthropic");
  });

  it("adds auth and resource headers for workspace proxy requests", () => {
    expect(buildProxyHeaders("token-123", "f/evals/ai/googleai")).toEqual({
      Authorization: "Bearer token-123",
      "X-Resource-Path": "f/evals/ai/googleai",
    });
  });
});

describe("resolveEvalModelProvider", () => {
  it("infers googleai from Gemini model ids", () => {
    expect(resolveEvalModelProvider("gemini-2.5-flash")).toEqual({
      provider: "googleai",
      model: "gemini-2.5-flash",
    });
  });

  it("preserves an explicit provider", () => {
    expect(resolveEvalModelProvider("gemini-2.5-pro", "googleai")).toEqual({
      provider: "googleai",
      model: "gemini-2.5-pro",
    });
  });
});
