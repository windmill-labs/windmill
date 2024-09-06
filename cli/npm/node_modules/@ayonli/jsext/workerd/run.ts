import { RunOptions, WorkerTask } from "../run.ts";
import parallel from "./parallel.ts";

export type { RunOptions, WorkerTask };

async function run<R, A extends any[] = any[]>(
    script: string,
    args?: A,
    options?: RunOptions
): Promise<WorkerTask<R>> {
    void script, args, options;
    throw new Error("Unsupported runtime");
}

namespace run {
    export var maxWorkers: number | undefined = undefined;
    /** @deprecated set {@link parallel.workerEntry} instead */
    export var workerEntry: string | undefined = undefined;
}
// backward compatibility
Object.defineProperties(run, {
    workerEntry: {
        set(v) {
            parallel.workerEntry = v;
        },
        get() {
            return parallel.workerEntry;
        },
    },
});

export default run;
