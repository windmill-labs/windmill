import { dir, ensureDir } from "./deps.ts";

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

export async function getRootStore(): Promise<string> {
  const store = (dir("config") ?? dir("tmp") ?? "/tmp/") + "/windmill/";
  await ensureDir(store);
  return store;
}

export async function getStore(baseUrl: string): Promise<string> {
  const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
  const baseStore = (await getRootStore()) + baseHash + "/";
  await ensureDir(baseStore);
  return baseStore;
}
