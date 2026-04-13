import { afterEach, describe, expect, it } from "bun:test";
import { getFrontendApiKey } from "./frontendCommon";

const ORIGINAL_ENV = {
  ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
  OPENAI_API_KEY: process.env.OPENAI_API_KEY,
  GEMINI_API_KEY: process.env.GEMINI_API_KEY,
};

afterEach(() => {
  process.env.ANTHROPIC_API_KEY = ORIGINAL_ENV.ANTHROPIC_API_KEY;
  process.env.OPENAI_API_KEY = ORIGINAL_ENV.OPENAI_API_KEY;
  process.env.GEMINI_API_KEY = ORIGINAL_ENV.GEMINI_API_KEY;
});

describe("getFrontendApiKey", () => {
  it("reads the Gemini API key for googleai models", () => {
    process.env.GEMINI_API_KEY = "gemini-test-key";
    expect(getFrontendApiKey("googleai")).toBe("gemini-test-key");
  });

  it("throws a provider-specific error when the key is missing", () => {
    delete process.env.GEMINI_API_KEY;
    expect(() => getFrontendApiKey("googleai")).toThrow(
      "GEMINI_API_KEY is required for frontend evals"
    );
  });
});
