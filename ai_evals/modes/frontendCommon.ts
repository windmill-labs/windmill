export const FRONTEND_PROVIDER = "anthropic";
export const FRONTEND_MODEL = "claude-haiku-4-5-20251001";

export function getFrontendApiKey(): string {
  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    throw new Error("ANTHROPIC_API_KEY is required for frontend evals");
  }
  return apiKey;
}

export function getFrontendRunModelLabel(): string {
  return `${FRONTEND_PROVIDER}:${FRONTEND_MODEL}`;
}
