import { query, type Options } from "@anthropic-ai/claude-agent-sdk";

export interface ToolInvocation {
  tool: string;
  input: Record<string, unknown>;
  timestamp: number;
}

export interface TestResult {
  toolsUsed: ToolInvocation[];
  skillsInvoked: string[];
  output: string;
}

/**
 * Runs a prompt through the Claude Agent SDK and captures tool invocations
 */
export async function runPromptAndCapture(
  prompt: string,
  cwd: string,
  maxTurns: number = 3
): Promise<TestResult> {
  const toolsUsed: ToolInvocation[] = [];
  const skillsInvoked: string[] = [];
  let output = "";

  const options: Options = {
    cwd,
    maxTurns,
    allowedTools: ["Skill", "Read", "Glob", "Grep", "Bash", "Write", "Edit"],
  };

  for await (const message of query({ prompt, options })) {
    if (message.type === "assistant") {
      // The assistant message has a BetaMessage which contains content blocks
      const content = message.message?.content;
      if (Array.isArray(content)) {
        for (const block of content) {
          if (block.type === "tool_use") {
            const toolInvocation: ToolInvocation = {
              tool: block.name,
              input: block.input as Record<string, unknown>,
              timestamp: Date.now(),
            };
            toolsUsed.push(toolInvocation);

            // Check if this is a Skill tool invocation
            if (block.name === "Skill" && typeof block.input === "object" && block.input !== null) {
              const skillInput = block.input as { skill?: string };
              if (skillInput.skill) {
                skillsInvoked.push(skillInput.skill);
              }
            }
          } else if (block.type === "text") {
            output += block.text;
          }
        }
      }
    } else if (message.type === "result") {
      // Capture final result if available
      const resultMessage = message as { result?: string };
      if (typeof resultMessage.result === "string") {
        output += resultMessage.result;
      }
    }
  }

  return {
    toolsUsed,
    skillsInvoked,
    output,
  };
}

/**
 * Helper to check if a specific tool was used
 */
export function wasToolUsed(result: TestResult, toolName: string): boolean {
  return result.toolsUsed.some((t) => t.tool === toolName);
}

/**
 * Helper to check if a specific skill was invoked
 */
export function wasSkillInvoked(result: TestResult, skillName: string): boolean {
  return result.skillsInvoked.some((s) => s === skillName || s.includes(skillName));
}

/**
 * Helper to get all tool inputs for a specific tool
 */
export function getToolInputs(result: TestResult, toolName: string): Record<string, unknown>[] {
  return result.toolsUsed.filter((t) => t.tool === toolName).map((t) => t.input);
}
