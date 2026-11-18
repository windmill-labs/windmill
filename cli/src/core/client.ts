import { OpenAPI } from "../../gen/index.ts";

/**
 * Mark every subsequent request as applying a state computed elsewhere rather
 * than authoring one in the target workspace (the fork tally reads the header to
 * decide whether a removal was deliberate).
 *
 * Belongs to the commands that apply and nothing that authors: `sync push`
 * (including the git-sync auto-pull, which runs it inside a job) and the
 * parent-to-fork half of `workspace merge`.
 */
export function markRequestsAsSyncOrigin() {
  const existing = typeof OpenAPI.HEADERS === "object" ? OpenAPI.HEADERS : {};
  OpenAPI.HEADERS = { ...existing, "X-Windmill-Deploy-Origin": "sync" };
}

/**
 * Name this process as the CLI on every subsequent request, so a trigger the
 * CLI created or disabled is attributed to `cli` rather than to a bare API
 * call in `trigger_history`. Attribution only — nothing on the server grants
 * anything on the strength of it.
 */
export function markRequestsAsCliClient() {
  const existing = typeof OpenAPI.HEADERS === "object" ? OpenAPI.HEADERS : {};
  OpenAPI.HEADERS = { ...existing, "X-Windmill-Client": "cli" };
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
