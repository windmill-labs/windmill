import { Command, Schedule } from "./deps.js";
export interface ScheduleFile {
    schedule: string;
    on_failure: string;
    script_path: string;
    args: any;
    timezone: string;
    is_flow: boolean;
}
export declare function pushSchedule(workspace: string, path: string, schedule: Schedule | ScheduleFile | undefined, localSchedule: ScheduleFile): Promise<void>;
declare const command: Command<void, void, void, [import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType, import("./deps/jsr.io/@windmill-labs/cliffy-command/1.0.0-rc.5/mod.js").StringType], void, void, void, Command<void, void, void, [], void, {
    number: number;
    integer: number;
    string: string;
    boolean: boolean;
    file: string;
}, void, undefined>>;
export default command;
//# sourceMappingURL=schedule.d.ts.map