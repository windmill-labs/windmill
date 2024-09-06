import { isMainThread } from '../env.js';

function parallel(module) {
    throw new Error("Unsupported runtime");
}
(function (parallel) {
    parallel.maxWorkers = undefined;
    parallel.workerEntry = undefined;
    parallel.isMainThread = false;
})(parallel || (parallel = {}));
Object.defineProperty(parallel, "isMainThread", {
    value: isMainThread,
    writable: false,
});
var parallel$1 = parallel;

export { parallel$1 as default };
//# sourceMappingURL=parallel.js.map
