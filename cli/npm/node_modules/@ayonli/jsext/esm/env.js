var _a, _b;
const id = Symbol.for("id");
const isBrowserWindow = typeof window === "object"
    && typeof window.document === "object"
    && typeof window.matchMedia === "function";
const isServiceWorker = typeof ServiceWorkerGlobalScope === "function"
    && globalThis instanceof ServiceWorkerGlobalScope;
const isSharedWorker = typeof SharedWorkerGlobalScope === "function"
    && globalThis instanceof SharedWorkerGlobalScope;
const isDedicatedWorker = typeof DedicatedWorkerGlobalScope === "function"
    && globalThis instanceof DedicatedWorkerGlobalScope;
const isWorker = isServiceWorker || isSharedWorker || isDedicatedWorker;
const isDeno = typeof Deno === "object" && !!((_a = Deno.version) === null || _a === void 0 ? void 0 : _a.deno);
const isBun = typeof Bun === "object" && !!Bun.version;
const isNodeLike = typeof process === "object" && !!((_b = process.versions) === null || _b === void 0 ? void 0 : _b.node) && !isDeno;
const isNode = isNodeLike && !isDeno && !isBun;
const isNodeBelow14 = isNode && parseInt(process.version.slice(1)) < 14;
const isNodeBelow16 = isNode && parseInt(process.version.slice(1)) < 16;
const isNodeBelow20 = isNode && parseInt(process.version.slice(1)) < 20;
const isNodeWorkerThread = isNode
    && (process.abort.disabled === true || process.argv.includes("--worker-thread"));
const isMainThread = !isNodeWorkerThread
    && (isBun ? Bun.isMainThread : typeof WorkerGlobalScope === "undefined");

export { id, isBrowserWindow, isBun, isDedicatedWorker, isDeno, isMainThread, isNode, isNodeBelow14, isNodeBelow16, isNodeBelow20, isNodeLike, isServiceWorker, isSharedWorker, isWorker };
//# sourceMappingURL=env.js.map
