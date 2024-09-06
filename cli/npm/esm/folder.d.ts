import { Command, Folder } from "./deps.js";
export interface FolderFile {
    owners: Array<string> | undefined;
    extra_perms: Map<string, boolean> | undefined;
    display_name: string | undefined;
}
export declare function pushFolder(workspace: string, name: string, folder: Folder | FolderFile | undefined, localFolder: FolderFile): Promise<void>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=folder.d.ts.map