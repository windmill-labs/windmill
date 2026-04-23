import { describe, expect, it } from "bun:test";
import {
  anthropicUsageToBenchmarkTokenUsage,
  extractCliResultTokenUsage,
  extractProposedWmillCommands,
  parseWmillInvocationLog,
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

describe("extractProposedWmillCommands", () => {
  it("extracts proposed commands from bullets, code blocks, and inline code", () => {
    expect(
      extractProposedWmillCommands(`
Next:
- \`wmill generate-metadata --yes\`
- wmill sync push

You can inspect failures with \`wmill job logs 123\`.
`)
    ).toEqual([
      "wmill generate-metadata --yes",
      "wmill sync push",
      "wmill job logs 123",
    ]);
  });

  it("extracts inline prose commands that are not wrapped in backticks", () => {
    expect(
      extractProposedWmillCommands(
        "The first command is wmill sync pull before you edit locally."
      )
    ).toEqual(["wmill sync pull"]);
  });

  it("extracts multiple inline prose commands from a single sentence", () => {
    expect(
      extractProposedWmillCommands(
        "Run wmill generate-metadata and then wmill sync push when you are ready."
      )
    ).toEqual(["wmill generate-metadata", "wmill sync push"]);
  });

  it("ignores negated command mentions", () => {
    expect(
      extractProposedWmillCommands(
        "Do not run `wmill sync push`. Instead run `wmill sync pull` first."
      )
    ).toEqual(["wmill sync pull"]);
  });
});

describe("parseWmillInvocationLog", () => {
  it("parses stubbed wmill invocations into structured records", () => {
    expect(
      parseWmillInvocationLog(`noise
__WMILL_BENCHMARK__
2026-04-21T12:00:00+00:00
/tmp/workspace
2
generate-metadata
--yes
__WMILL_BENCHMARK__
2026-04-21T12:00:05+00:00
/tmp/workspace
3
sync
push
--dry-run
`)
    ).toEqual([
      {
        argv: ["generate-metadata", "--yes"],
        cwd: "/tmp/workspace",
        timestamp: "2026-04-21T12:00:00+00:00",
      },
      {
        argv: ["sync", "push", "--dry-run"],
        cwd: "/tmp/workspace",
        timestamp: "2026-04-21T12:00:05+00:00",
      },
    ]);
  });
});
