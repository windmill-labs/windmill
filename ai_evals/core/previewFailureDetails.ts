export interface PreviewFailureDetailsInput {
  workspaceId: string;
  jobId: string;
  logs?: string | null;
  result?: unknown;
  maxCharacters?: number;
}

export function formatPreviewFailureDetails(
  input: PreviewFailureDetailsInput,
): string {
  const parts = [`workspace=${input.workspaceId}`, `job=${input.jobId}`];
  const maxCharacters = input.maxCharacters ?? 1200;
  // A job log starts with dependency install output and ends with what ran just
  // before the failure: keep its tail.
  const logExcerpt = normalizeExcerpt(input.logs, maxCharacters, "tail");
  const resultExcerpt = normalizeExcerpt(
    formatResult(input.result),
    maxCharacters,
    "head",
  );

  if (logExcerpt) {
    parts.push(`logs=${logExcerpt}`);
  }
  if (resultExcerpt) {
    parts.push(`result=${resultExcerpt}`);
  }

  return parts.join("; ");
}

function formatResult(result: unknown): string | null {
  if (result == null) {
    return null;
  }
  if (typeof result === "string") {
    return result;
  }
  // A failed job's error serializes its message after the stack, which a long stack
  // would push out of the excerpt.
  const error = (
    result as { error?: { name?: unknown; message?: unknown; stack?: unknown } }
  ).error;
  if (error && typeof error === "object" && typeof error.message === "string") {
    const name = typeof error.name === "string" ? `${error.name}: ` : "";
    const stack = typeof error.stack === "string" ? ` ${error.stack}` : "";
    return `${name}${error.message}${stack}`;
  }
  try {
    return JSON.stringify(result);
  } catch {
    return String(result);
  }
}

function normalizeExcerpt(
  value: string | null | undefined,
  maxCharacters: number,
  keep: "head" | "tail",
): string | null {
  const normalized = value?.replace(/\s+/g, " ").trim();
  if (!normalized) {
    return null;
  }
  if (normalized.length <= maxCharacters) {
    return normalized;
  }
  return keep === "tail"
    ? `…${normalized.slice(normalized.length - (maxCharacters - 1))}`
    : `${normalized.slice(0, maxCharacters - 1)}…`;
}
