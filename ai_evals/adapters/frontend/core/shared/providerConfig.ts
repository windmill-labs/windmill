import Anthropic from "@anthropic-ai/sdk";
import OpenAI from "openai";
import type { FrontendEvalModelConfig } from "../../../../core/models";
import type { FrontendEvalTransport } from "../../../../core/frontendTransport";

export type FrontendEvalProvider = FrontendEvalModelConfig["provider"];

export interface EvalClients {
  openai: OpenAI;
  anthropic: Anthropic;
}

export interface ResolvedEvalModelProvider {
  provider: FrontendEvalProvider;
  model: string;
}

export interface EvalProxyClientConfig {
  baseURL: string;
  bearerToken: string;
  resourcePath: string;
}

const GEMINI_OPENAI_BASE_URL =
  "https://generativelanguage.googleapis.com/v1beta/openai/";
const GEMINI_GOOG_API_CLIENT = "windmill-ai-evals/1.0";
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

export function buildOpenAICompatibleClientOptions(
  provider: Exclude<FrontendEvalProvider, "anthropic">,
  apiKey: string,
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

function buildProxyOpenAIClientOptions(
  proxy: EvalProxyClientConfig,
): ConstructorParameters<typeof OpenAI>[0] {
  return {
    apiKey: "unused",
    baseURL: proxy.baseURL,
    defaultHeaders: buildProxyHeaders(proxy.bearerToken, proxy.resourcePath),
  };
}

export function createEvalClients(input: {
  provider: FrontendEvalProvider;
  apiKey: string;
  transport?: FrontendEvalTransport;
  proxy?: EvalProxyClientConfig;
}): EvalClients {
  const transport = input.transport ?? "direct";

  if (input.provider === "anthropic") {
    if (transport === "proxy") {
      if (!input.proxy) {
        throw new Error(
          "Missing proxy client configuration for proxy transport",
        );
      }
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
      openai: new OpenAI({ apiKey: "unused" }),
      anthropic: new Anthropic({ apiKey: input.apiKey }),
    };
  }

  if (transport === "proxy") {
    if (!input.proxy) {
      throw new Error("Missing proxy client configuration for proxy transport");
    }
    return {
      openai: new OpenAI(buildProxyOpenAIClientOptions(input.proxy)),
      anthropic: new Anthropic({ apiKey: "unused" }),
    };
  }

  return {
    openai: new OpenAI(
      buildOpenAICompatibleClientOptions(input.provider, input.apiKey),
    ),
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
  if (model.startsWith("gpt") || model.startsWith("o")) {
    return { provider: "openai", model };
  }
  return { provider: "openai", model };
}
