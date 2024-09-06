export const id = Symbol.for("id");

declare const Bun: any;
declare const WorkerGlobalScope: Function | undefined;
declare const DedicatedWorkerGlobalScope: Function | undefined;
declare const ServiceWorkerGlobalScope: Function | undefined;
declare const SharedWorkerGlobalScope: Function | undefined;

export const isBrowserWindow = typeof window === "object"
    && typeof window.document === "object"
    && typeof window.matchMedia === "function";
export const isServiceWorker = typeof ServiceWorkerGlobalScope === "function"
    && globalThis instanceof ServiceWorkerGlobalScope;
export const isSharedWorker = typeof SharedWorkerGlobalScope === "function"
    && globalThis instanceof SharedWorkerGlobalScope;
export const isDedicatedWorker = typeof DedicatedWorkerGlobalScope === "function"
    && globalThis instanceof DedicatedWorkerGlobalScope;
export const isWorker = isServiceWorker || isSharedWorker || isDedicatedWorker;

export const isDeno = typeof Deno === "object" && !!Deno.version?.deno;
export const isBun = typeof Bun === "object" && !!Bun.version;
export const isNodeLike = typeof process === "object" && !!process.versions?.node && !isDeno;
export const isNode: boolean = isNodeLike && !isDeno && !isBun;

export const isNodeBelow14 = isNode && parseInt(process.version.slice(1)) < 14;
export const isNodeBelow16 = isNode && parseInt(process.version.slice(1)) < 16;
export const isNodeBelow20 = isNode && parseInt(process.version.slice(1)) < 20;

const isNodeWorkerThread = isNode
    && ((process.abort as any).disabled === true || process.argv.includes("--worker-thread"));
export const isMainThread = !isNodeWorkerThread
    && (isBun ? (Bun.isMainThread as boolean) : typeof WorkerGlobalScope === "undefined");
