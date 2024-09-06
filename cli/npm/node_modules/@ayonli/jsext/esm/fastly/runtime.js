import { env as env$1 } from 'fastly:env';
export { WellknownPlatforms, WellknownRuntimes, addShutdownListener, addUnhandledRejectionListener, customInspect, default, isREPL, platform, refTimer, unrefTimer } from '../runtime.js';

// @ts-ignore
function env(name = undefined, value = undefined) {
    var _a, _b, _c, _d, _e, _f, _g, _h;
    if (typeof name === "object") {
        throw new Error("Not implemented");
    }
    else if (value !== undefined) {
        throw new Error("Cannot modify environment variables in the worker");
    }
    else if (name === undefined) {
        return {
            "FASTLY_CACHE_GENERATION": (_a = env$1("FASTLY_CACHE_GENERATION")) !== null && _a !== void 0 ? _a : "",
            "FASTLY_CUSTOMER_ID": (_b = env$1("FASTLY_CUSTOMER_ID")) !== null && _b !== void 0 ? _b : "",
            "FASTLY_HOSTNAME": (_c = env$1("FASTLY_HOSTNAME")) !== null && _c !== void 0 ? _c : "",
            "FASTLY_POP": (_d = env$1("FASTLY_POP")) !== null && _d !== void 0 ? _d : "",
            "FASTLY_REGION": (_e = env$1("FASTLY_REGION")) !== null && _e !== void 0 ? _e : "",
            "FASTLY_SERVICE_ID": (_f = env$1("FASTLY_SERVICE_ID")) !== null && _f !== void 0 ? _f : "",
            "FASTLY_SERVICE_VERSION": (_g = env$1("FASTLY_SERVICE_VERSION")) !== null && _g !== void 0 ? _g : "",
            "FASTLY_TRACE_ID": (_h = env$1("FASTLY_TRACE_ID")) !== null && _h !== void 0 ? _h : "",
        };
    }
    return env$1(name);
}

export { env };
//# sourceMappingURL=runtime.js.map
