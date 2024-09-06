export interface SyncOptions {
    stateful?: boolean;
    raw?: boolean;
    yes?: boolean;
    skipPull?: boolean;
    failConflicts?: boolean;
    plainSecrets?: boolean;
    json?: boolean;
    skipVariables?: boolean;
    skipResources?: boolean;
    skipSecrets?: boolean;
    includeSchedules?: boolean;
    includeUsers?: boolean;
    includeGroups?: boolean;
    includeSettings?: boolean;
    includeKey?: boolean;
    message?: string;
    includes?: string[];
    extraIncludes?: string[];
    excludes?: string[];
    defaultTs?: "bun" | "deno";
    codebases?: Codebase[];
}
export interface Codebase {
    relative_path: string;
    includes?: string[];
    excludes?: string[];
    assets?: {
        from: string;
        to: string;
    }[];
    customBundler?: string;
    external?: string[];
    define?: {
        [key: string]: string;
    };
    inject?: string[];
}
export declare function readConfigFile(): Promise<SyncOptions>;
export declare function mergeConfigWithConfigFile<T>(opts: T): Promise<T & SyncOptions>;
//# sourceMappingURL=conf.d.ts.map