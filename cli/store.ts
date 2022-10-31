import dir from "https://deno.land/x/dir@1.5.1/mod.ts";
import * as fs from "https://deno.land/std@0.161.0/fs/mod.ts";

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

export async function getStore(baseUrl: string): Promise<string> {
  const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
  const store = dir("config") + "/windmill/";
  await fs.ensureDir(store);
  const baseStore = store + baseHash + "/";
  await fs.ensureDir(baseStore);
  return baseStore;
}
