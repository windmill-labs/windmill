import { mkdir } from "node:fs/promises";
import { getConfigDirPath } from "../../windmill-utils-internal/src/config/config.ts";

function hash_string(str: string): number {
  let hash = 0,
    i,
    chr;
  if (str.length === 0) return hash;
  for (i = 0; i < str.length; i++) {
    chr = str.charCodeAt(i);
    hash = (hash << 5) - hash + chr;
    hash |= 0; // Convert to 32bit integer
  }
  return hash;
}

export async function getStore(baseUrl: string, configDirOverride?: string): Promise<string> {
  const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
  const baseStore = (await getConfigDirPath(configDirOverride)) + baseHash + "/";
  await mkdir(baseStore, { recursive: true });
  return baseStore;
}