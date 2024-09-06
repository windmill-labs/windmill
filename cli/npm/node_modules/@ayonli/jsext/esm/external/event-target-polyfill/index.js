var _a;
if (typeof globalThis.Event !== "function") {
    // @ts-ignore
    globalThis.Event = (_a = class Event {
            constructor(type, eventInitDict = {}) {
                this.type = type;
                this.eventInitDict = eventInitDict;
                this.bubbles = false;
                this.cancelable = false;
                this.cancelBubble = false;
                this.composed = false;
                this.currentTarget = null;
                this.defaultPrevented = false;
                this.eventPhase = _a.NONE;
                this.isTrusted = false;
                this.returnValue = true;
                this.target = null;
                this.timeStamp = Date.now();
                this.srcElement = null;
                this.AT_TARGET = 2;
                this.BUBBLING_PHASE = 3;
                this.CAPTURING_PHASE = 1;
                this.NONE = 0;
                if (eventInitDict.bubbles !== undefined) {
                    this.bubbles = eventInitDict.bubbles;
                }
                if (eventInitDict.cancelable !== undefined) {
                    this.cancelable = eventInitDict.cancelable;
                }
                if (eventInitDict.composed !== undefined) {
                    this.composed = eventInitDict.composed;
                }
            }
            composedPath() {
                return [];
            }
            preventDefault() {
                if (this.cancelable) {
                    this.defaultPrevented = true;
                }
            }
            stopImmediatePropagation() {
                // Do nothing
            }
            stopPropagation() {
                this.cancelBubble = true;
            }
            initEvent(type, bubbles = undefined, cancelable = undefined) {
                this.type = type;
                this.bubbles = bubbles !== null && bubbles !== void 0 ? bubbles : false;
                this.cancelable = cancelable !== null && cancelable !== void 0 ? cancelable : false;
            }
        },
        _a.AT_TARGET = 2,
        _a.BUBBLING_PHASE = 3,
        _a.CAPTURING_PHASE = 1,
        _a.NONE = 0,
        _a);
}
if (typeof globalThis.EventTarget !== "function") {
    // @ts-ignore
    globalThis.EventTarget = class EventTarget {
        constructor() {
            this.listeners = {};
        }
        addEventListener(type, callback, options = {}) {
            var _b;
            if (!(type in this.listeners)) {
                this.listeners[type] = [];
            }
            // @ts-ignore
            this.listeners[type].push({ callback, once: (_b = options === null || options === void 0 ? void 0 : options.once) !== null && _b !== void 0 ? _b : false });
        }
        removeEventListener(type, callback) {
            if (!(type in this.listeners)) {
                return;
            }
            const stack = this.listeners[type];
            for (let i = 0, l = stack.length; i < l; i++) {
                if (stack[i].callback === callback) {
                    stack.splice(i, 1);
                    return;
                }
            }
            if (stack.length === 0) {
                delete this.listeners[type];
            }
        }
        dispatchEvent(event) {
            if (!(event.type in this.listeners)) {
                return true;
            }
            Object.defineProperties(event, {
                currentTarget: { configurable: true, value: this },
                target: { configurable: true, value: this },
            });
            const stack = this.listeners[event.type].slice();
            for (let i = 0, l = stack.length; i < l; i++) {
                const listener = stack[i];
                try {
                    listener.callback.call(this, event);
                }
                catch (err) {
                    setTimeout(() => {
                        throw err;
                    });
                }
                if (listener.once) {
                    this.removeEventListener(event.type, listener.callback);
                }
            }
            return !event.defaultPrevented;
        }
    };
}
//# sourceMappingURL=index.js.map
