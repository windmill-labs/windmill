import type { FrontendEvalModelConfig } from "../core/models";

export function getFrontendApiKey(
  provider: FrontendEvalModelConfig["provider"],
): string {
  const envName =
    provider === "anthropic"
      ? "ANTHROPIC_API_KEY"
      : provider === "googleai"
        ? "GEMINI_API_KEY"
        : provider === "deepseek"
          ? "DEEPSEEK_API_KEY"
          : "OPENAI_API_KEY";
  const apiKey = process.env[envName];
  if (!apiKey) {
    throw new Error(`${envName} is required for frontend evals`);
  }
  return apiKey;
}
