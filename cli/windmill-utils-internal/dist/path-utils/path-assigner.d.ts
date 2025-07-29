import { RawScript } from "../gen/types.gen";
/**
 * Union type of all supported programming languages in Windmill
 */
export type SupportedLanguage = RawScript["language"] | "frontend" | "bunnative" | "oracledb" | "rust" | "csharp" | "nu" | "ansible" | "java" | "duckdb";
/**
 * Mapping of supported languages to their file extensions
 */
export declare const LANGUAGE_EXTENSIONS: Record<SupportedLanguage, string>;
/**
 * Gets the appropriate file extension for a given programming language.
 * Handles special cases for TypeScript variants based on the default runtime.
 *
 * @param language - The programming language to get extension for
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns File extension string (without the dot)
 */
export declare function getLanguageExtension(language: SupportedLanguage, defaultTs?: "bun" | "deno"): string;
/**
 * Assigns a file path for an inline script based on its ID and language.
 * Returns both the base path and extension as separate components.
 *
 * @param id - Unique identifier for the script
 * @param language - Programming language of the script
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 * @returns Tuple containing [basePath, extension]
 */
export declare function assignPath(id: string, language: SupportedLanguage, defaultTs?: "bun" | "deno"): [string, string];
