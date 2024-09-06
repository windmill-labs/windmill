import { Command, JSZip } from "./deps.js";
import { Workspace } from "./workspace.js";
export declare function downloadZip(workspace: Workspace, plainSecrets: boolean | undefined, skipVariables?: boolean, skipResources?: boolean, skipSecrets?: boolean, includeSchedules?: boolean, includeUsers?: boolean, includeGroups?: boolean, includeSettings?: boolean, includeKey?: boolean, defaultTs?: "bun" | "deno"): Promise<JSZip | undefined>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType & string], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>;
export default command;
//# sourceMappingURL=pull.d.ts.map