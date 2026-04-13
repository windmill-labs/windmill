import Anthropic from "@anthropic-ai/sdk";
import OpenAI from "openai";
import type { FrontendEvalModelConfig } from "../../../../core/models";

export type FrontendEvalProvider = FrontendEvalModelConfig["provider"];

export interface EvalClients {
  openai: OpenAI;
  anthropic: Anthropic;
}

export interface ResolvedEvalModelProvider {
  provider: FrontendEvalProvider;
  model: string;
}

const GEMINI_OPENAI_BASE_URL = "https://generativelanguage.googleapis.com/v1beta/openai/";
const GEMINI_GOOG_API_CLIENT = "windmill-ai-evals/1.0";

export function buildOpenAICompatibleClientOptions(
  provider: Exclude<FrontendEvalProvider, "anthropic">,
  apiKey: string
): ConstructorParameters<typeof OpenAI>[0] {
  if (provider === "googleai") {
    return {
      apiKey,
      baseURL: GEMINI_OPENAI_BASE_URL,
      defaultHeaders: {
        "x-goog-api-client": GEMINI_GOOG_API_CLIENT,
      },
    };
  }

  return { apiKey };
}

export function createEvalClients(
  provider: FrontendEvalProvider,
  apiKey: string
): EvalClients {
  if (provider === "anthropic") {
    return {
      openai: new OpenAI({ apiKey: "unused" }),
      anthropic: new Anthropic({ apiKey }),
    };
  }

  return {
    openai: new OpenAI(buildOpenAICompatibleClientOptions(provider, apiKey)),
    anthropic: new Anthropic({ apiKey: "unused" }),
  };
}

export function resolveEvalModelProvider(
  model: string,
  provider?: FrontendEvalProvider
): ResolvedEvalModelProvider {
  if (provider) {
    return { provider, model };
  }
  if (model.startsWith("claude")) {
    return { provider: "anthropic", model };
  }
  if (model.startsWith("gemini")) {
    return { provider: "googleai", model };
  }
  if (model.startsWith("gpt") || model.startsWith("o")) {
    return { provider: "openai", model };
  }
  return { provider: "openai", model };
}
