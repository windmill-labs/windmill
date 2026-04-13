import { describe, expect, it } from "bun:test";
import {
  buildOpenAICompatibleClientOptions,
  resolveEvalModelProvider,
} from "./providerConfig";

describe("buildOpenAICompatibleClientOptions", () => {
  it("adds Gemini's OpenAI-compatible base URL and client header", () => {
    const options = buildOpenAICompatibleClientOptions("googleai", "gemini-test-key");

    expect(options).toMatchObject({
      apiKey: "gemini-test-key",
      baseURL: "https://generativelanguage.googleapis.com/v1beta/openai/",
      defaultHeaders: {
        "x-goog-api-client": "windmill-ai-evals/1.0",
      },
    });
  });

  it("keeps the default OpenAI-compatible config for OpenAI", () => {
    expect(buildOpenAICompatibleClientOptions("openai", "openai-test-key")).toEqual({
      apiKey: "openai-test-key",
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
