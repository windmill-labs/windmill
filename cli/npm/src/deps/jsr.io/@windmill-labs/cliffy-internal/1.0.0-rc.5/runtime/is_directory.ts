import { stat } from "./stat.js";

/**
 * Check if given path is a directory.
 *
 * @internal
 * @param path The file path.
 */
export async function isDirectory(path: string): Promise<boolean> {
  try {
    const { isDirectory } = await stat(path);
    return isDirectory;
  } catch {
    return false;
  }
}
