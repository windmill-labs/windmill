import { GlobalOptions } from "./types.js";
import { FlowValue } from "./deps.js";
import { Workspace } from "./workspace.js";
import { SchemaProperty } from "./bootstrap/common.js";
import { ScriptLanguage } from "./script_common.js";
import { GlobalDeps } from "./script.js";
import { SyncCodebase } from "./codebase.js";
export declare function generateAllMetadata(): Promise<void>;
export declare function generateFlowLockInternal(folder: string, dryRun: boolean, workspace: Workspace, justUpdateMetadataLock?: boolean): Promise<string | undefined>;
export declare function generateScriptMetadataInternal(scriptPath: string, workspace: Workspace, opts: GlobalOptions & {
    lockOnly?: boolean | undefined;
    schemaOnly?: boolean | undefined;
    defaultTs?: "bun" | "deno";
}, dryRun: boolean, noStaleMessage: boolean, globalDeps: GlobalDeps, codebases: SyncCodebase[], justUpdateMetadataLock?: boolean): Promise<string | undefined>;
export declare function updateScriptSchema(scriptContent: string, language: ScriptLanguage, metadataContent: Record<string, any>, path: string): Promise<void>;
export declare function updateFlow(workspace: Workspace, flow_value: FlowValue, remotePath: string): Promise<FlowValue | undefined>;
export declare function inferSchema(language: ScriptLanguage, content: string, currentSchema: any, path: string): any;
export declare function argSigToJsonSchemaType(t: string | {
    resource: string | null;
} | {
    list: string | {
        str: any;
    } | {
        object: {
            key: string;
            typ: any;
        }[];
    } | null;
} | {
    str: string[] | null;
} | {
    object: {
        key: string;
        typ: any;
    }[];
} | {
    oneof: [
        {
            label: string;
            properties: {
                key: string;
                typ: any;
            }[];
        }
    ];
}, oldS: SchemaProperty): void;
export declare function replaceLock(o?: {
    lock?: string | string[];
}): void;
export declare function parseMetadataFile(scriptPath: string, generateMetadataIfMissing: (GlobalOptions & {
    path: string;
    workspaceRemote: Workspace;
    schemaOnly?: boolean;
}) | undefined, globalDeps: GlobalDeps, codebases: SyncCodebase[]): Promise<{
    isJson: boolean;
    payload: any;
    path: string;
}>;
interface Lock {
    locks?: {
        [path: string]: string | {
            [subpath: string]: string;
        };
    };
}
export declare function readLockfile(): Promise<Lock>;
export declare function checkifMetadataUptodate(path: string, hash: string, conf: Lock | undefined, subpath?: string): Promise<boolean>;
export declare function generateScriptHash(rawReqs: string | undefined, scriptContent: string, newMetadataContent: string): Promise<string>;
export declare function updateMetadataGlobalLock(path: string, hash: string, subpath?: string): Promise<void>;
export {};
//# sourceMappingURL=metadata.d.ts.map