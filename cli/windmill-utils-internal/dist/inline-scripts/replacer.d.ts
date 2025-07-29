import { FlowModule } from "../gen/types.gen";
/**
 * Replaces inline script references with actual file content from the filesystem.
 * This function recursively processes all flow modules and their nested structures.
 *
 * @param modules - Array of flow modules to process
 * @param fileReader - Function to read file content (typically fs.readFile or similar)
 * @param logger - Optional logger object with info and error methods
 * @param localPath - Base path for resolving relative file paths
 * @param removeLocks - Optional array of paths for which to remove lock files
 * @returns Promise that resolves when all inline scripts have been replaced
 */
export declare function replaceInlineScripts(modules: FlowModule[], fileReader: (path: string) => Promise<string>, logger: {
    info: (message: string) => void;
    error: (message: string) => void;
} | undefined, localPath: string, separator?: string, removeLocks?: string[], renamer?: (path: string, newPath: string) => void, deleter?: (path: string) => void): Promise<void>;
