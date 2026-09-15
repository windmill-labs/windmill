import { randomUUID } from "node:crypto";

const DEFAULT_WORKSPACE_PREFIX = "ai-evals";

// A workspace id must be at most 50 characters AND match `^\w+(-\w+)*$`
// (`workspace.proper_id`), so the case slug yields to the random suffix that
// makes the id unique, and the truncation must not leave a trailing hyphen the
// suffix would double.
const MAX_WORKSPACE_ID_LENGTH = 50;

export function buildWorkspaceId(caseId: string, attempt: number): string {
  const caseSlug = caseId
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  const suffix = `-a${attempt}-${randomUUID().slice(0, 8)}`;
  const head = `${DEFAULT_WORKSPACE_PREFIX}-${caseSlug || "case"}`;
  return `${head
    .slice(0, MAX_WORKSPACE_ID_LENGTH - suffix.length)
    .replace(/-+$/, "")}${suffix}`;
}
