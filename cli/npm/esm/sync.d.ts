import { Command, FlowModule } from "./deps.js";
import { GlobalOptions } from "./types.js";
import { SyncOptions } from "./conf.js";
import { SyncCodebase } from "./codebase.js";
type DynFSElement = {
    isDirectory: boolean;
    path: string;
    getContentText(): Promise<string>;
    getChildren(): AsyncIterable<DynFSElement>;
};
export declare function findCodebase(path: string, codebases: SyncCodebase[]): SyncCodebase | undefined;
export declare function FSFSElement(p: string, codebases: SyncCodebase[]): Promise<DynFSElement>;
export declare const yamlOptions: {
    sortKeys: (a: any, b: any) => number;
    noCompatMode: boolean;
    noRefs: boolean;
    skipInvalid: boolean;
};
export interface InlineScript {
    path: string;
    content: string;
}
export declare function extractInlineScriptsForFlows(modules: FlowModule[], pathAssigner: PathAssigner): InlineScript[];
interface PathAssigner {
    assignPath(summary: string | undefined, language: string): [string, string];
}
export declare function extractInlineScriptsForApps(rec: any, pathAssigner: PathAssigner): InlineScript[];
export declare function newPathAssigner(defaultTs: "bun" | "deno"): PathAssigner;
export declare function readDirRecursiveWithIgnore(ignore: (path: string, isDirectory: boolean) => boolean, root: DynFSElement): AsyncGenerator<{
    path: string;
    ignored: boolean;
    isDirectory: boolean;
    getContentText(): Promise<string>;
}>;
export declare function elementsToMap(els: DynFSElement, ignore: (path: string, isDirectory: boolean) => boolean, json: boolean, skips: Skips): Promise<{
    [key: string]: string;
}>;
interface Skips {
    skipVariables?: boolean | undefined;
    skipResources?: boolean | undefined;
    skipSecrets?: boolean | undefined;
    includeSchedules?: boolean | undefined;
    includeUsers?: boolean | undefined;
    includeGroups?: boolean | undefined;
    includeSettings?: boolean | undefined;
    includeKey?: boolean | undefined;
}
export declare const isWhitelisted: (p: string) => boolean;
export declare function ignoreF(wmillconf: {
    includes?: string[];
    excludes?: string[];
}): Promise<(p: string, isDirectory: boolean) => boolean>;
export declare function pull(opts: GlobalOptions & SyncOptions): Promise<void>;
export declare function push(opts: GlobalOptions & SyncOptions): Promise<void>;
declare const command: Command<void, void, {
    failConflicts?: true | undefined;
} & {
    raw?: true | undefined;
} & {
    stateful?: true | undefined;
} & {
    skipPull?: true | undefined;
} & {
    yes?: true | undefined;
} & {
    plainSecrets?: true | undefined;
} & {
    json?: true | undefined;
} & {
    skipVariables?: true | undefined;
} & {
    skipSecrets?: true | undefined;
} & {
    skipResources?: true | undefined;
} & {
    includeSchedules?: true | undefined;
} & {
    includeUsers?: true | undefined;
} & {
    includeGroups?: true | undefined;
} & {
    includeSettings?: true | undefined;
} & {
    includeKey?: true | undefined;
} & {
    includes?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").FileType[] | undefined;
} & {
    excludes?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").FileType[] | undefined;
} & {
    message?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType | undefined;
}, [], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=sync.d.ts.map