import { GlobalUserInfo } from "./deps.js";
import { GlobalOptions } from "./types.js";
import { Workspace } from "./workspace.js";
export type Context = {
    workspace: string;
    baseUrl: string;
    urlStore: string;
    token: string;
};
export declare function resolveWorkspace(opts: GlobalOptions): Promise<Workspace>;
export declare function requireLogin(opts: GlobalOptions): Promise<GlobalUserInfo>;
export declare function fetchVersion(baseUrl: string): Promise<string>;
export declare function tryResolveVersion(opts: GlobalOptions): Promise<number | undefined>;
export declare function validatePath(path: string): boolean;
//# sourceMappingURL=context.d.ts.map