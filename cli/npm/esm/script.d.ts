import { GlobalOptions } from "./types.js";
import { Command } from "./deps.js";
import { Workspace } from "./workspace.js";
import { ScriptLanguage } from "./script_common.js";
import { SyncCodebase } from "./codebase.js";
export interface ScriptFile {
    parent_hash?: string;
    summary: string;
    description: string;
    schema?: any;
    is_template?: boolean;
    lock?: Array<string>;
    kind?: "script" | "failure" | "trigger" | "command" | "approval";
}
export declare function handleScriptMetadata(path: string, workspace: Workspace, alreadySynced: string[], message: string | undefined, globalDeps: GlobalDeps, codebases: SyncCodebase[], opts: GlobalOptions): Promise<boolean>;
export declare function handleFile(path: string, workspace: Workspace, alreadySynced: string[], message: string | undefined, opts: (GlobalOptions & {
    defaultTs?: "bun" | "deno";
}) | undefined, globalDeps: GlobalDeps, codebases: SyncCodebase[]): Promise<boolean>;
export declare function findContentFile(filePath: string): Promise<string>;
export declare function filePathExtensionFromContentType(language: ScriptLanguage, defaultTs: "bun" | "deno" | undefined): string;
export declare const exts: string[];
export declare function removeExtensionToPath(path: string): string;
export declare function resolve(input: string): Promise<Record<string, any>>;
export declare function track_job(workspace: string, id: string): Promise<void>;
export type GlobalDeps = {
    pkgs: Record<string, string>;
    reqs: Record<string, string>;
    composers: Record<string, string>;
};
export declare function findGlobalDeps(): Promise<GlobalDeps>;
declare const command: Command<void, void, {
    yes?: true | undefined;
} & {
    lockOnly?: true | undefined;
} & {
    schemaOnly?: true | undefined;
} & {
    includes?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").FileType[] | undefined;
} & {
    excludes?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").FileType[] | undefined;
}, [(import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").FileType | undefined)?], void, void, void, Command<void, void, {
    showArchived?: true | undefined;
}, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=script.d.ts.map