import { Command, ListableVariable } from "./deps.js";
export interface VariableFile {
    value: string;
    is_secret: boolean;
    description: string;
    account?: number;
    is_oauth?: boolean;
}
export declare function pushVariable(workspace: string, remotePath: string, variable: VariableFile | ListableVariable | undefined, localVariable: VariableFile, plainSecrets: boolean): Promise<void>;
declare const command: Command<void, void, {
    public?: true | undefined;
}, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=variable.d.ts.map