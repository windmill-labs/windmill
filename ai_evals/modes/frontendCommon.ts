import {
  getFrontendEvalModel,
  resolveEvalModel,
  type FrontendEvalModelConfig,
} from "../core/models";

export const DEFAULT_FRONTEND_EVAL_MODEL: FrontendEvalModelConfig = getFrontendEvalModel(
  resolveEvalModel("flow")
);

export function getFrontendApiKey(provider: FrontendEvalModelConfig["provider"]): string {
  const apiKey =
    provider === "anthropic" ? process.env.ANTHROPIC_API_KEY : process.env.OPENAI_API_KEY;
  if (!apiKey) {
    const envName = provider === "anthropic" ? "ANTHROPIC_API_KEY" : "OPENAI_API_KEY";
    throw new Error(`${envName} is required for frontend evals`);
  }
  return apiKey;
}

export function getFrontendRunModelLabel(model: FrontendEvalModelConfig): string {
  return `${model.provider}:${model.model}`;
}
