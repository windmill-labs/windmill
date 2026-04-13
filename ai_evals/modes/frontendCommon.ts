import {
  getFrontendEvalModel,
  resolveEvalModel,
  type FrontendEvalModelConfig,
} from "../core/models";

export const DEFAULT_FRONTEND_EVAL_MODEL: FrontendEvalModelConfig = getFrontendEvalModel(
  resolveEvalModel("flow")
);

export function getFrontendApiKey(provider: FrontendEvalModelConfig["provider"]): string {
  const envName =
    provider === "anthropic"
      ? "ANTHROPIC_API_KEY"
      : provider === "googleai"
        ? "GEMINI_API_KEY"
        : "OPENAI_API_KEY";
  const apiKey = process.env[envName];
  if (!apiKey) {
    throw new Error(`${envName} is required for frontend evals`);
  }
  return apiKey;
}
