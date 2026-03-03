import * as log from "./log.ts";
import { readFile, writeFile } from "node:fs/promises";
import { getStore } from "./store.ts";

export interface BranchProfileMapping {
  lastUsed: {
    [key: string]: string; // key format: "branch|baseUrl|workspaceId" -> profile name
  };
}

const BRANCH_PROFILES_FILE = "branch-profiles.json";

export async function getBranchProfilesPath(configDirOverride?: string): Promise<string> {
  return (await getStore("", configDirOverride)) + BRANCH_PROFILES_FILE;
}

export async function loadBranchProfiles(configDirOverride?: string): Promise<BranchProfileMapping> {
  try {
    const path = await getBranchProfilesPath(configDirOverride);
    const content = await readFile(path, "utf-8");
    return JSON.parse(content);
  } catch {
    // File doesn't exist or invalid JSON - return empty mapping
    return { lastUsed: {} };
  }
}

export async function saveBranchProfiles(
  mapping: BranchProfileMapping,
  configDirOverride?: string
): Promise<void> {
  const path = await getBranchProfilesPath(configDirOverride);
  await writeFile(path, JSON.stringify(mapping, null, 2), "utf-8");
}

export function getBranchProfileKey(
  branch: string,
  baseUrl: string,
  workspaceId: string
): string {
  // Normalize baseUrl to ensure consistency
  let normalizedUrl: string;
  try {
    normalizedUrl = new URL(baseUrl).toString();
  } catch {
    // Fallback to non-normalized URL if parsing fails
    normalizedUrl = baseUrl;
  }
  return `${branch}|${normalizedUrl}|${workspaceId}`;
}

export async function getLastUsedProfile(
  branch: string,
  baseUrl: string,
  workspaceId: string,
  configDirOverride?: string
): Promise<string | undefined> {
  const mapping = await loadBranchProfiles(configDirOverride);
  const key = getBranchProfileKey(branch, baseUrl, workspaceId);
  return mapping.lastUsed[key];
}

export async function setLastUsedProfile(
  branch: string,
  baseUrl: string,
  workspaceId: string,
  profileName: string,
  configDirOverride?: string
): Promise<void> {
  const mapping = await loadBranchProfiles(configDirOverride);
  const key = getBranchProfileKey(branch, baseUrl, workspaceId);
  mapping.lastUsed[key] = profileName;
  await saveBranchProfiles(mapping, configDirOverride);
  log.debug(`Saved last used profile for ${key}: ${profileName}`);
}
