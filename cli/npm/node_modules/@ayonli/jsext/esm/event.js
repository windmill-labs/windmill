import './external/event-target-polyfill/index.js';

/**
 * Functions for working with events.
 * @module
 */
function createErrorEvent(type, options = {}) {
    var _a, _b, _c, _d, _e, _f, _g, _h;
    if (typeof ErrorEvent === "function") {
        return new ErrorEvent(type, options);
    }
    else {
        const event = new Event(type, {
            bubbles: (_a = options === null || options === void 0 ? void 0 : options.bubbles) !== null && _a !== void 0 ? _a : false,
            cancelable: (_b = options === null || options === void 0 ? void 0 : options.cancelable) !== null && _b !== void 0 ? _b : false,
            composed: (_c = options === null || options === void 0 ? void 0 : options.composed) !== null && _c !== void 0 ? _c : false,
        });
        Object.defineProperties(event, {
            message: { configurable: true, value: (_d = options === null || options === void 0 ? void 0 : options.message) !== null && _d !== void 0 ? _d : "" },
            filename: { configurable: true, value: (_e = options === null || options === void 0 ? void 0 : options.filename) !== null && _e !== void 0 ? _e : "" },
            lineno: { configurable: true, value: (_f = options === null || options === void 0 ? void 0 : options.lineno) !== null && _f !== void 0 ? _f : 0 },
            colno: { configurable: true, value: (_g = options === null || options === void 0 ? void 0 : options.colno) !== null && _g !== void 0 ? _g : 0 },
            error: { configurable: true, value: (_h = options === null || options === void 0 ? void 0 : options.error) !== null && _h !== void 0 ? _h : undefined },
        });
        return event;
    }
}
function createCloseEvent(type, options = {}) {
    var _a, _b, _c, _d, _e, _f;
    if (typeof CloseEvent === "function") {
        return new CloseEvent(type, options);
    }
    else {
        const event = new Event(type, {
            bubbles: (_a = options === null || options === void 0 ? void 0 : options.bubbles) !== null && _a !== void 0 ? _a : false,
            cancelable: (_b = options === null || options === void 0 ? void 0 : options.cancelable) !== null && _b !== void 0 ? _b : false,
            composed: (_c = options === null || options === void 0 ? void 0 : options.composed) !== null && _c !== void 0 ? _c : false,
        });
        Object.defineProperties(event, {
            code: { configurable: true, value: (_d = options.code) !== null && _d !== void 0 ? _d : 0 },
            reason: { configurable: true, value: (_e = options.reason) !== null && _e !== void 0 ? _e : "" },
            wasClean: { configurable: true, value: (_f = options.wasClean) !== null && _f !== void 0 ? _f : false },
        });
        return event;
    }
}
function createProgressEvent(type, options = {}) {
    var _a, _b, _c, _d, _e, _f;
    if (typeof ProgressEvent === "function") {
        return new ProgressEvent(type, options);
    }
    else {
        const event = new Event(type, {
            bubbles: (_a = options === null || options === void 0 ? void 0 : options.bubbles) !== null && _a !== void 0 ? _a : false,
            cancelable: (_b = options === null || options === void 0 ? void 0 : options.cancelable) !== null && _b !== void 0 ? _b : false,
            composed: (_c = options === null || options === void 0 ? void 0 : options.composed) !== null && _c !== void 0 ? _c : false,
        });
        Object.defineProperties(event, {
            lengthComputable: { configurable: true, value: (_d = options === null || options === void 0 ? void 0 : options.lengthComputable) !== null && _d !== void 0 ? _d : false },
            loaded: { configurable: true, value: (_e = options === null || options === void 0 ? void 0 : options.loaded) !== null && _e !== void 0 ? _e : 0 },
            total: { configurable: true, value: (_f = options === null || options === void 0 ? void 0 : options.total) !== null && _f !== void 0 ? _f : 0 },
        });
        return event;
    }
}
/**
 * Creates a `CustomEvent` instance based on the given options. If the
 * `CustomEvent` constructor is not available, the generic `Event` constructor
 * will be used instead, and the options will be attached to the event as its
 * properties.
 */
function createCustomEvent(type, options = {}) {
    var _a, _b, _c, _d;
    if (typeof CustomEvent === "function") {
        return new CustomEvent(type, options);
    }
    else {
        const event = new Event(type, {
            bubbles: (_a = options === null || options === void 0 ? void 0 : options.bubbles) !== null && _a !== void 0 ? _a : false,
            cancelable: (_b = options === null || options === void 0 ? void 0 : options.cancelable) !== null && _b !== void 0 ? _b : false,
            composed: (_c = options === null || options === void 0 ? void 0 : options.composed) !== null && _c !== void 0 ? _c : false,
        });
        Object.defineProperties(event, {
            detail: { configurable: true, value: (_d = options === null || options === void 0 ? void 0 : options.detail) !== null && _d !== void 0 ? _d : null },
        });
        return event;
    }
}

export { createCloseEvent, createCustomEvent, createErrorEvent, createProgressEvent };
//# sourceMappingURL=event.js.map
