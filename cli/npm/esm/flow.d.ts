import { GlobalOptions } from "./types.js";
import { Command, FlowModule } from "./deps.js";
export interface FlowFile {
    summary: string;
    description?: string;
    value: any;
    schema?: any;
}
export declare function replaceInlineScripts(modules: FlowModule[], localPath: string, removeLocks: string[] | undefined): void;
export declare function pushFlow(workspace: string, remotePath: string, localPath: string, message?: string): Promise<void>;
export declare function bootstrap(opts: GlobalOptions & {
    summary: string;
    description: string;
}, flowPath: string): void;
declare const command: Command<void, void, {
    summary?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType | undefined;
} & {
    description?: import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType | undefined;
}, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, {
    showArchived?: true | undefined;
}, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=flow.d.ts.map