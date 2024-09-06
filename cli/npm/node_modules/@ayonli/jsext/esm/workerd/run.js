import parallel from './parallel.js';

async function run(script, args, options) {
    throw new Error("Unsupported runtime");
}
(function (run) {
    run.maxWorkers = undefined;
    /** @deprecated set {@link parallel.workerEntry} instead */
    run.workerEntry = undefined;
})(run || (run = {}));
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
var run$1 = run;

export { run$1 as default };
//# sourceMappingURL=run.js.map
