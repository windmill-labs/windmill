import { FlowModule } from "../gen/types.gen";
/**
 * Represents an inline script extracted from a flow module
 */
interface InlineScript {
    /** File path where the script content should be written */
    path: string;
    /** The actual script content */
    content: string;
}
/**
 * Extracts inline scripts from flow modules, converting them to separate files
 * and replacing the original content with file references.
 *
 * @param modules - Array of flow modules to process
 * @param mapping - Optional mapping of module IDs to custom file paths
 * @param defaultTs - Default TypeScript runtime to use ("bun" or "deno")
 * @returns Array of inline scripts with their paths and content
 */
export declare function extractInlineScripts(modules: FlowModule[], mapping?: Record<string, string>, separator?: string, defaultTs?: "bun" | "deno"): InlineScript[];
/**
 * Extracts the current mapping of module IDs to file paths from flow modules
 * by analyzing existing inline script references.
 *
 * @param modules - Array of flow modules to analyze (can be undefined)
 * @param mapping - Existing mapping to extend (defaults to empty object)
 * @returns Record mapping module IDs to their corresponding file paths
 */
export declare function extractCurrentMapping(modules: FlowModule[] | undefined, mapping?: Record<string, string>): Record<string, string>;
export {};
