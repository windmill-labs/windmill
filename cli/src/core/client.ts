import { OpenAPI } from "../../gen/index.ts";

/**
 * Mark every subsequent request as applying a state computed elsewhere rather
 * than authoring one in the target workspace (the fork tally reads the header to
 * decide whether a removal was deliberate).
 *
 * Belongs to `sync push` and nothing that authors: it is always the applying
 * half, including the git-sync auto-pull, which runs it inside a job.
 */
export function markRequestsAsSyncOrigin() {
  const existing = typeof OpenAPI.HEADERS === "object" ? OpenAPI.HEADERS : {};
  OpenAPI.HEADERS = { ...existing, "X-Windmill-Deploy-Origin": "sync" };
}

export function setClient(token?: string, baseUrl?: string) {
  if (baseUrl === undefined) {
    baseUrl = process.env["BASE_INTERNAL_URL"] ??
      process.env["BASE_URL"] ??
      "http://localhost:8000";
  }
  if (token === undefined) {
    token = process.env["WM_TOKEN"] ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}
