import { Command, Policy } from "./deps.js";
export interface AppFile {
    value: any;
    summary: string;
    policy: Policy;
}
export declare function pushApp(workspace: string, remotePath: string, localPath: string, message?: string): Promise<void>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=apps.d.ts.map