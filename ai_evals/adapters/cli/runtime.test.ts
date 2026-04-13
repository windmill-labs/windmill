import { describe, expect, it } from "bun:test";
import {
  anthropicUsageToBenchmarkTokenUsage,
  extractCliResultTokenUsage,
} from "./runtime";

describe("anthropicUsageToBenchmarkTokenUsage", () => {
  it("includes cache tokens in prompt usage", () => {
    expect(
      anthropicUsageToBenchmarkTokenUsage({
        input_tokens: 120,
        output_tokens: 45,
        cache_creation_input_tokens: 30,
        cache_read_input_tokens: 5,
      })
    ).toEqual({
      prompt: 155,
      completion: 45,
      total: 200,
    });
  });

  it("returns null when usage is absent", () => {
    expect(anthropicUsageToBenchmarkTokenUsage(null)).toBeNull();
  });
});

describe("extractCliResultTokenUsage", () => {
  it("reads aggregate usage from the SDK result event", () => {
    expect(
      extractCliResultTokenUsage({
        type: "result",
        usage: {
          input_tokens: 400,
          output_tokens: 120,
          cache_creation_input_tokens: 50,
          cache_read_input_tokens: 25,
        },
      })
    ).toEqual({
      prompt: 475,
      completion: 120,
      total: 595,
    });
  });

  it("falls back to modelUsage when aggregate usage is unavailable", () => {
    expect(
      extractCliResultTokenUsage({
        type: "result",
        modelUsage: {
          opus: {
            inputTokens: 200,
            outputTokens: 60,
            cacheCreationInputTokens: 10,
            cacheReadInputTokens: 5,
          },
          haiku: {
            inputTokens: 80,
            outputTokens: 20,
            cacheCreationInputTokens: 0,
            cacheReadInputTokens: 15,
          },
        },
      })
    ).toEqual({
      prompt: 310,
      completion: 80,
      total: 390,
    });
  });
});
