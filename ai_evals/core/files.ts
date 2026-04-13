import { access, copyFile, mkdir, readdir, readFile } from "node:fs/promises";
import path from "node:path";

export async function exists(filePath: string): Promise<boolean> {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

export async function readJsonFile<T>(filePath: string): Promise<T> {
  const raw = await readFile(filePath, "utf8");
  return JSON.parse(raw) as T;
}

export async function readDirectoryFiles(
  rootDir: string,
  options: {
    ignore?: Set<string>;
  } = {}
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};
  await walkDirectory(rootDir, "", files, options.ignore ?? new Set());
  return files;
}

export async function copyDirectory(sourceDir: string, targetDir: string): Promise<void> {
  const entries = await readdir(sourceDir, { withFileTypes: true });
  await mkdir(targetDir, { recursive: true });

  for (const entry of entries) {
    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);
    if (entry.isDirectory()) {
      await copyDirectory(sourcePath, targetPath);
      continue;
    }
    await mkdir(path.dirname(targetPath), { recursive: true });
    await copyFile(sourcePath, targetPath);
  }
}

async function walkDirectory(
  absoluteDir: string,
  relativeDir: string,
  output: Record<string, string>,
  ignore: Set<string>
): Promise<void> {
  const entries = await readdir(absoluteDir, { withFileTypes: true });

  for (const entry of entries) {
    const relativePath = relativeDir ? `${relativeDir}/${entry.name}` : entry.name;
    if (ignore.has(relativePath) || ignore.has(entry.name)) {
      continue;
    }

    const absolutePath = path.join(absoluteDir, entry.name);
    if (entry.isDirectory()) {
      await walkDirectory(absolutePath, relativePath, output, ignore);
      continue;
    }

    output[relativePath] = await readFile(absolutePath, "utf8");
  }
}
