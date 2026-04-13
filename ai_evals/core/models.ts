import type { EvalMode } from "./types";

export interface FrontendEvalModelConfig {
  provider: "anthropic" | "openai" | "googleai";
  model: string;
}

export interface CliEvalModelConfig {
  provider: "anthropic";
  model: string;
}

export interface EvalModelSpec {
  id: string;
  label: string;
  aliases: string[];
  frontend?: FrontendEvalModelConfig;
  cli?: CliEvalModelConfig;
}

export const EVAL_MODELS: EvalModelSpec[] = [
  {
    id: "haiku",
    label: "Claude Haiku 4.5",
    aliases: [
      "haiku",
      "haiku-4.5",
      "claude-haiku",
      "claude-haiku-4.5",
      "claude-haiku-4-5",
      "claude-haiku-4-5-20251001",
    ],
    frontend: {
      provider: "anthropic",
      model: "claude-haiku-4-5-20251001",
    },
    cli: {
      provider: "anthropic",
      model: "haiku",
    },
  },
  {
    id: "sonnet",
    label: "Claude Sonnet 4.5",
    aliases: [
      "sonnet",
      "sonnet-4.5",
      "claude-sonnet",
      "claude-sonnet-4.5",
      "claude-sonnet-4-5",
      "claude-sonnet-4-5-20250929",
    ],
    frontend: {
      provider: "anthropic",
      model: "claude-sonnet-4-5-20250929",
    },
    cli: {
      provider: "anthropic",
      model: "sonnet",
    },
  },
  {
    id: "opus",
    label: "Claude Opus 4.6",
    aliases: [
      "opus",
      "opus-4.6",
      "claude-opus",
      "claude-opus-4.6",
      "claude-opus-4-6",
    ],
    frontend: {
      provider: "anthropic",
      model: "claude-opus-4-6",
    },
    cli: {
      provider: "anthropic",
      model: "opus",
    },
  },
  {
    id: "4o",
    label: "GPT-4o",
    aliases: ["4o", "gpt-4o"],
    frontend: {
      provider: "openai",
      model: "gpt-4o",
    },
  },
  {
    id: "gemini-flash",
    label: "Gemini 2.5 Flash",
    aliases: ["gemini", "gemini-flash", "gemini-2.5-flash"],
    frontend: {
      provider: "googleai",
      model: "gemini-2.5-flash",
    },
  },
  {
    id: "gemini-pro",
    label: "Gemini 2.5 Pro",
    aliases: ["gemini-pro", "gemini-2.5-pro"],
    frontend: {
      provider: "googleai",
      model: "gemini-2.5-pro",
    },
  },
  {
    id: "gemini-3-flash-preview",
    label: "Gemini 3 Flash Preview",
    aliases: ["gemini-3-flash-preview", "gemini-3-flash"],
    frontend: {
      provider: "googleai",
      model: "gemini-3-flash-preview",
    },
  },
  {
    id: "gemini-3.1-pro-preview",
    label: "Gemini 3.1 Pro Preview",
    aliases: ["gemini-3.1-pro-preview", "gemini-3.1-pro", "gemini-3-pro-preview"],
    frontend: {
      provider: "googleai",
      model: "gemini-3.1-pro-preview",
    },
  },
];

export function resolveEvalModel(mode: EvalMode, alias?: string): EvalModelSpec {
  const spec = alias ? findEvalModel(alias) : getDefaultEvalModel(mode);
  if (!spec) {
    throw new Error(`Unknown model: ${alias}`);
  }

  if (mode === "cli" && !spec.cli) {
    throw new Error(`Model ${spec.id} is not supported for cli mode`);
  }

  if (mode !== "cli" && !spec.frontend) {
    throw new Error(`Model ${spec.id} is not supported for ${mode} mode`);
  }

  return spec;
}

export function getEvalModelHelpText(): string {
  return EVAL_MODELS.map((model) => {
    const modes = [
      ...(model.frontend ? ["flow", "script", "app"] : []),
      ...(model.cli ? ["cli"] : []),
    ];
    return `  ${model.id.padEnd(8)} ${model.label} (${modes.join(", ")})`;
  }).join("\n");
}

export function formatRunModelLabel(mode: EvalMode, model: EvalModelSpec): string {
  if (mode === "cli") {
    return `${model.cli!.provider}:${model.cli!.model}`;
  }
  return `${model.frontend!.provider}:${model.frontend!.model}`;
}

export function getFrontendEvalModel(model: EvalModelSpec): FrontendEvalModelConfig {
  if (!model.frontend) {
    throw new Error(`Model ${model.id} does not support frontend evals`);
  }
  return model.frontend;
}

export function getCliEvalModel(model: EvalModelSpec): CliEvalModelConfig {
  if (!model.cli) {
    throw new Error(`Model ${model.id} does not support cli evals`);
  }
  return model.cli;
}

function getDefaultEvalModel(mode: EvalMode): EvalModelSpec {
  return mode === "cli" ? EVAL_MODELS[0]! : EVAL_MODELS[0]!;
}

function findEvalModel(alias: string): EvalModelSpec | undefined {
  const normalized = alias.trim().toLowerCase();
  return EVAL_MODELS.find((model) =>
    [model.id, ...model.aliases].some((candidate) => candidate.toLowerCase() === normalized)
  );
}
