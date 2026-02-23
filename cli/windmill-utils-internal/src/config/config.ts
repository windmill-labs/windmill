import { stat, mkdir } from "node:fs/promises";

export const WINDMILL_CONFIG_DIR = "windmill";
export const WINDMILL_ACTIVE_WORKSPACE_FILE = "activeWorkspace";
export const WINDMILL_WORKSPACE_CONFIG_FILE = "remotes.ndjson";
export const INSTANCES_CONFIG_FILE = "instances.ndjson";
export const WINDMILL_ACTIVE_INSTANCE_FILE = "activeInstance";

function getEnv(key: string): string | undefined {
  return process.env[key];
}

function getOS(): "linux" | "darwin" | "windows" | null {
  const platform = process.platform;
  switch (platform) {
    case "linux": return "linux";
    case "darwin": return "darwin";
    case "win32": return "windows";
    default: return null;
  }
}

function throwIfNotDirectory(fileInfo: import("node:fs").Stats): void {
  if (!fileInfo.isDirectory()) {
    throw new Error("Path is not a directory");
  }
}

function config_dir(): string | null {
  const os = getOS();
  switch (os) {
    case "linux": {
      const xdg = getEnv("XDG_CONFIG_HOME");
      if (xdg) return xdg;

      const home = getEnv("HOME");
      if (home) return `${home}/.config`;
      break;
    }

    case "darwin": {
      const home = getEnv("HOME");
      if (home) return `${home}/Library/Preferences`;
      break;
    }

    case "windows":
      return getEnv("APPDATA") ?? null;
  }

  return null;
}

function tmp_dir(): string | null {
  const os = getOS();
  switch (os) {
    case "linux": {
      const xdg = getEnv("XDG_RUNTIME_DIR");
      if (xdg) return `${xdg}-/tmp`;

      const tmpDir = getEnv("TMPDIR");
      if (tmpDir) return tmpDir;

      const tempDir = getEnv("TEMP");
      if (tempDir) return tempDir;

      const tmp = getEnv("TMP");
      if (tmp) return tmp;

      return "/var/tmp";
    }
    case "darwin":
      return getEnv("TMPDIR") ?? null;
    case "windows":
      return getEnv("TMP") ?? getEnv("TEMP") ?? null;
  }
  return null;
}

async function ensureDir(dir: string | URL) {
  try {
    const fileInfo = await stat(dir);
    throwIfNotDirectory(fileInfo);
    return;
  } catch (err: any) {
    if (err.code !== 'ENOENT') {
      throw err;
    }
  }

  // The dir doesn't exist. Create it.
  // This can be racy. So we catch AlreadyExists and check stat again.
  try {
    await mkdir(dir, { recursive: true });
  } catch (err: any) {
    if (err.code !== 'EEXIST') {
      throw err;
    }

    const fileInfo = await stat(dir);
    throwIfNotDirectory(fileInfo);
  }
}

export async function getBaseConfigDir(configDirOverride?: string): Promise<string> {
  const baseDir = configDirOverride ??
                  getEnv("WMILL_CONFIG_DIR") ??
                  config_dir() ??
                  tmp_dir() ??
                  "/tmp/";
  return baseDir;
}

export async function getConfigDirPath(configDirOverride?: string): Promise<string> {
  const baseDir = await getBaseConfigDir(configDirOverride);
  const store = baseDir + `/${WINDMILL_CONFIG_DIR}/`;
  await ensureDir(store);
  return store;
}

export async function getWorkspaceConfigFilePath(configDirOverride?: string): Promise<string> {
  const configDir = await getConfigDirPath(configDirOverride);
  return `${configDir}/${WINDMILL_WORKSPACE_CONFIG_FILE}`;
}

export async function getActiveWorkspaceConfigFilePath(configDirOverride?: string): Promise<string> {
  const configDir = await getConfigDirPath(configDirOverride);
  return `${configDir}/${WINDMILL_ACTIVE_WORKSPACE_FILE}`;
}

export async function getInstancesConfigFilePath(configDirOverride?: string): Promise<string> {
  const configDir = await getConfigDirPath(configDirOverride);
  return `${configDir}/${INSTANCES_CONFIG_FILE}`;
}

export async function getActiveInstanceFilePath(configDirOverride?: string): Promise<string> {
  const configDir = await getConfigDirPath(configDirOverride);
  return `${configDir}/${WINDMILL_ACTIVE_INSTANCE_FILE}`;
}
