import { describe, expect, it } from "bun:test";
import { resolveEvalModel } from "./models";

describe("resolveEvalModel", () => {
  it("supports Gemini aliases for frontend evals", () => {
    expect(resolveEvalModel("flow", "gemini").frontend).toEqual({
      provider: "googleai",
      model: "gemini-2.5-flash",
    });
    expect(resolveEvalModel("app", "gemini-pro").frontend).toEqual({
      provider: "googleai",
      model: "gemini-2.5-pro",
    });
    expect(resolveEvalModel("script", "gemini-3-flash-preview").frontend).toEqual({
      provider: "googleai",
      model: "gemini-3-flash-preview",
    });
    expect(resolveEvalModel("flow", "gemini-3.1-pro-preview").frontend).toEqual({
      provider: "googleai",
      model: "gemini-3.1-pro-preview",
    });
  });

  it("rejects Gemini aliases for cli evals", () => {
    expect(() => resolveEvalModel("cli", "gemini")).toThrow(
      "Model gemini-flash is not supported for cli mode"
    );
  });
});
