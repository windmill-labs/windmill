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

export interface WindmillAiProxyClientConfig {
  baseURL: string;
  bearerToken: string;
  resourcePath: string;
}

const EVAL_PROXY_RESOURCE_PREFIX = "f/evals/ai";

export function buildProxyHeaders(
  bearerToken: string,
  resourcePath: string,
): Record<string, string> {
  return {
    Authorization: `Bearer ${bearerToken}`,
    "X-Resource-Path": resourcePath,
  };
}

export function buildProxyResourcePath(provider: FrontendEvalProvider): string {
  return `${EVAL_PROXY_RESOURCE_PREFIX}/${provider}`;
}

function buildProxyOpenAIClientOptions(
  proxy: WindmillAiProxyClientConfig,
): ConstructorParameters<typeof OpenAI>[0] {
  return {
    apiKey: "unused",
    baseURL: proxy.baseURL,
    defaultHeaders: buildProxyHeaders(proxy.bearerToken, proxy.resourcePath),
  };
}

export function createEvalClients(input: {
  provider: FrontendEvalProvider;
  proxy: WindmillAiProxyClientConfig;
}): EvalClients {
  if (input.provider === "anthropic") {
    return {
      openai: new OpenAI({ apiKey: "unused" }),
      anthropic: new Anthropic({
        apiKey: "unused",
        baseURL: input.proxy.baseURL,
        defaultHeaders: buildProxyHeaders(
          input.proxy.bearerToken,
          input.proxy.resourcePath,
        ),
      }),
    };
  }

  return {
    openai: new OpenAI(buildProxyOpenAIClientOptions(input.proxy)),
    anthropic: new Anthropic({ apiKey: "unused" }),
  };
}

export function resolveEvalModelProvider(
  model: string,
  provider?: FrontendEvalProvider,
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
  if (model.startsWith("deepseek")) {
    return { provider: "deepseek", model };
  }
  if (model.startsWith("gpt") || model.startsWith("o")) {
    return { provider: "openai", model };
  }
  return { provider: "openai", model };
}
