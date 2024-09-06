import { GlobalOptions } from "./types.js";
import { Command } from "./deps.js";
export interface Workspace {
    remote: string;
    workspaceId: string;
    name: string;
    token: string;
}
export declare function allWorkspaces(): Promise<Workspace[]>;
export declare function getActiveWorkspace(opts: GlobalOptions): Promise<Workspace | undefined>;
export declare function getWorkspaceByName(workspaceName: string): Promise<Workspace | undefined>;
export declare function add(opts: GlobalOptions & {
    create: boolean;
    createWorkspaceName: string | undefined;
    createUsername: string | undefined;
}, workspaceName: string | undefined, workspaceId: string | undefined, remote: string | undefined): Promise<void>;
export declare function addWorkspace(workspace: Workspace, opts: any): Promise<void>;
export declare function removeWorkspace(name: string, silent: boolean, opts: any): Promise<void>;
declare const command: Command<void, void, void, [], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=workspace.d.ts.map