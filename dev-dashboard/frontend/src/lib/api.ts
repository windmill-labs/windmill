import type { WorktreeInfo } from "./types";

async function api<T = unknown>(path: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(`/api/${path}`, {
    headers: { "Content-Type": "application/json" },
    ...opts,
  });
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data as T;
}

export function fetchWorktrees(): Promise<WorktreeInfo[]> {
  return api<WorktreeInfo[]>("worktrees");
}

export type Profile = "full" | "agent-only";

export function createWorktree(
  branch: string,
  profile: Profile = "agent-only",
): Promise<unknown> {
  return api("worktrees", {
    method: "POST",
    body: JSON.stringify({ branch, profile }),
  });
}

export function removeWorktree(name: string): Promise<unknown> {
  return api(`worktrees/${encodeURIComponent(name)}`, { method: "DELETE" });
}

export function openWorktree(name: string): Promise<unknown> {
  return api(`worktrees/${encodeURIComponent(name)}/open`, { method: "POST" });
}

export function closeWorktree(name: string): Promise<unknown> {
  return api(`worktrees/${encodeURIComponent(name)}/close`, { method: "POST" });
}

export function sendPrompt(name: string, prompt: string): Promise<unknown> {
  return api(`worktrees/${encodeURIComponent(name)}/send`, {
    method: "POST",
    body: JSON.stringify({ prompt }),
  });
}
