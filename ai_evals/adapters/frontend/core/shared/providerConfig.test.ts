import { describe, expect, it } from "bun:test";
import {
  buildProxyHeaders,
  buildProxyResourcePath,
  resolveEvalModelProvider,
} from "./providerConfig";

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
    expect(resolveEvalModelProvider("gemini-3-flash-preview")).toEqual({
      provider: "googleai",
      model: "gemini-3-flash-preview",
    });
  });

  it("infers deepseek from DeepSeek model ids", () => {
    expect(resolveEvalModelProvider("deepseek-v4-flash")).toEqual({
      provider: "deepseek",
      model: "deepseek-v4-flash",
    });
  });

  it("preserves an explicit provider", () => {
    expect(
      resolveEvalModelProvider("gemini-3.1-pro-preview", "googleai"),
    ).toEqual({
      provider: "googleai",
      model: "gemini-3.1-pro-preview",
    });
  });
});
