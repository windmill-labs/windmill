import { Codebase, SyncOptions } from "./conf.js";
export type SyncCodebase = Codebase & {
    digest: string;
};
export declare function listSyncCodebases(options: SyncOptions): Promise<SyncCodebase[]>;
//# sourceMappingURL=codebase.d.ts.map