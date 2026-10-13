import { describe, expect, it } from "bun:test";
import { formatRunModelLabel, resolveEvalModel } from "./models";

describe("resolveEvalModel", () => {
  it("supports GPT-5.5 aliases for frontend evals", () => {
    expect(resolveEvalModel("flow", "gpt-5.5").frontend).toEqual({
      provider: "openai",
      model: "gpt-5.5",
    });
    expect(resolveEvalModel("app", "gpt-55").frontend).toEqual({
      provider: "openai",
      model: "gpt-5.5",
    });
    expect(resolveEvalModel("script", "5.5").frontend).toEqual({
      provider: "openai",
      model: "gpt-5.5",
    });
  });

  it("supports Gemini aliases for frontend evals", () => {
    expect(
      resolveEvalModel("script", "gemini-3-flash-preview").frontend,
    ).toEqual({
      provider: "googleai",
      model: "gemini-3-flash-preview",
    });
    expect(resolveEvalModel("flow", "gemini-3.1-pro-preview").frontend).toEqual(
      {
        provider: "googleai",
        model: "gemini-3.1-pro-preview",
      },
    );
  });

  it("supports DeepSeek aliases for frontend evals", () => {
    expect(resolveEvalModel("flow", "deepseek").frontend).toEqual({
      provider: "deepseek",
      model: "deepseek-flash",
    });
    expect(resolveEvalModel("flow", "deepseek-v4-flash").frontend).toEqual({
      provider: "deepseek",
      model: "deepseek-v4-flash",
    });
    expect(resolveEvalModel("script", "deepseek-v4-pro").frontend).toEqual({
      provider: "deepseek",
      model: "deepseek-v4-pro",
    });
  });

  it("labels cli runs by agent, keeping Claude's historical label", () => {
    expect(formatRunModelLabel("cli", resolveEvalModel("cli", "sonnet-5.5"))).toBe(
      "anthropic:claude-sonnet-5-5",
    );
    expect(formatRunModelLabel("cli", resolveEvalModel("cli", "gpt-6.1-sol"))).toBe(
      "codex:gpt-6.1-sol",
    );
    expect(formatRunModelLabel("cli", resolveEvalModel("cli", "deepseek-v4-pro"))).toBe(
      "claude-code:deepseek-v4-pro",
    );
  });

  it("rejects Gemini aliases for cli evals", () => {
    expect(() => resolveEvalModel("cli", "gemini-3-flash-preview")).toThrow(
      "Model gemini-3-flash-preview is not supported for cli mode",
    );
  });
});
