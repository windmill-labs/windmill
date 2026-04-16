import Anthropic from "@anthropic-ai/sdk";
import type { EvalMode, JudgeResult } from "./types";

export const DEFAULT_JUDGE_MODEL = "claude-sonnet-4-6";

const JUDGE_TOOL_NAME = "submit_judgement";

export async function judgeOutput(input: {
  mode: EvalMode;
  prompt: string;
  checklist?: string[];
  initial?: unknown;
  expected?: unknown;
  actual: unknown;
  model?: string;
}): Promise<JudgeResult> {
  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    return {
      success: false,
      score: 0,
      summary: "Judge unavailable",
      error: "ANTHROPIC_API_KEY is not set",
    };
  }

  const client = new Anthropic({ apiKey });
  const model = input.model ?? DEFAULT_JUDGE_MODEL;

  const system = [
    "You evaluate benchmark outputs for Windmill AI generation.",
    "Deterministic checks already run separately. Focus on whether the final output satisfies the user request.",
    "If expected state is provided, treat it as a valid example and reward semantically equivalent outputs.",
    "If a checklist is provided, treat it as the explicit acceptance criteria for this case.",
    "Be strict about missing requested functionality.",
    "When the prompt wording is ambiguous, prefer the checklist over inferred structural requirements.",
    "Do not invent additional Windmill-specific constraints that are not explicit in the prompt, checklist, or expected state.",
    "Do not lower the score just because the output uses a different but valid Windmill idiom, naming choice, or equivalent field shape.",
    "Do not require exact ids, exact topology, or exact field names unless the prompt, checklist, or expected state clearly requires them.",
    `Always respond by calling the ${JUDGE_TOOL_NAME} tool exactly once.`,
  ].join("\n\n");

  const user = [
    `Mode: ${input.mode}`,
    "",
    "User prompt:",
    input.prompt,
    "",
    "Checklist:",
    formatChecklist(input.checklist),
    "",
    "Initial state:",
    formatJsonBlock(input.initial),
    "",
    "Expected state:",
    formatJsonBlock(input.expected),
    "",
    "Actual result:",
    formatJsonBlock(input.actual),
  ].join("\n");

  try {
    const response = await client.messages.create({
      model,
      max_tokens: 1024,
      temperature: 0,
      system,
      messages: [{ role: "user", content: user }],
      tools: [
        {
          name: JUDGE_TOOL_NAME,
          description: "Submit the benchmark judgement as structured data.",
          input_schema: {
            type: "object",
            properties: {
              score: {
                type: "integer",
                minimum: 0,
                maximum: 100,
              },
              summary: {
                type: "string",
              },
            },
            required: ["score", "summary"],
          },
        },
      ],
      tool_choice: {
        type: "tool",
        name: JUDGE_TOOL_NAME,
        disable_parallel_tool_use: true,
      },
    });

    const toolUseBlock = response.content.find(
      (block): block is Anthropic.ToolUseBlock =>
        block.type === "tool_use" && block.name === JUDGE_TOOL_NAME
    );

    if (!toolUseBlock) {
      return {
        success: false,
        score: 0,
        summary: "Judge returned no tool output",
        error: "Expected structured tool output from judge",
      };
    }

    const parsed = toolUseBlock.input as {
      score: number;
      summary: string;
    };

    return {
      success: true,
      score: normalizeScore(parsed.score),
      summary: parsed.summary,
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      score: 0,
      summary: "Judge failed",
      error: message,
    };
  }
}

function formatJsonBlock(value: unknown): string {
  if (value === undefined) {
    return "(none)";
  }
  return JSON.stringify(value, null, 2);
}

function formatChecklist(checklist: string[] | undefined): string {
  if (!checklist || checklist.length === 0) {
    return "(none)";
  }

  return checklist.map((item) => `- ${item}`).join("\n");
}

function normalizeScore(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(100, Math.round(value)));
}
