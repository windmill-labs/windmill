import { ensureDir } from "../../deps.ts";

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

export async function getRootStore(configDirOverride?: string): Promise<string> {
  const baseDir = configDirOverride ?? 
                  Deno.env.get("WMILL_CONFIG_DIR") ?? 
                  config_dir() ?? 
                  tmp_dir() ?? 
                  "/tmp/";
  const store = baseDir + "/windmill/";
  await ensureDir(store);
  return store;
}

export async function getStore(baseUrl: string, configDirOverride?: string): Promise<string> {
  const baseHash = Math.abs(hash_string(baseUrl)).toString(16);
  const baseStore = (await getRootStore(configDirOverride)) + baseHash + "/";
  await ensureDir(baseStore);
  return baseStore;
}

//inlined import dir from "https://deno.land/x/dir/mod.ts";
function tmp_dir(): string | null {
  switch (Deno.build.os) {
    case "linux": {
      const xdg = Deno.env.get("XDG_RUNTIME_DIR");
      if (xdg) return `${xdg}-/tmp`;

      const tmpDir = Deno.env.get("TMPDIR");
      if (tmpDir) return tmpDir;

      const tempDir = Deno.env.get("TEMP");
      if (tempDir) return tempDir;

      const tmp = Deno.env.get("TMP");
      if (tmp) return tmp;

      return "/var/tmp";
    }
    case "darwin":
      return Deno.env.get("TMPDIR") ?? null;
    case "windows":
      return Deno.env.get("TMP") ?? Deno.env.get("TEMP") ?? null;
  }
  return null;
}

function config_dir(): string | null {
  switch (Deno.build.os) {
    case "linux": {
      const xdg = Deno.env.get("XDG_CONFIG_HOME");
      if (xdg) return xdg;

      const home = Deno.env.get("HOME");
      if (home) return `${home}/.config`;
      break;
    }

    case "darwin": {
      const home = Deno.env.get("HOME");
      if (home) return `${home}/Library/Preferences`;
      break;
    }

    case "windows":
      return Deno.env.get("APPDATA") ?? null;
  }

  return null;
}
