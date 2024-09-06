if (typeof globalThis.Event !== "function") {
    // @ts-ignore
    globalThis.Event = class Event implements globalThis.Event {
        bubbles = false;
        cancelable = false;
        cancelBubble = false;
        composed = false;
        currentTarget: EventTarget | null = null;
        defaultPrevented = false;
        eventPhase = Event.NONE;
        isTrusted = false;
        returnValue = true;
        target: EventTarget | null = null;
        timeStamp = Date.now();
        srcElement: EventTarget | null = null;

        AT_TARGET = 2 as const;
        BUBBLING_PHASE = 3 as const;
        CAPTURING_PHASE = 1 as const;
        NONE = 0 as const;

        static AT_TARGET = 2 as const;
        static BUBBLING_PHASE = 3 as const;
        static CAPTURING_PHASE = 1 as const;
        static NONE = 0 as const;

        constructor(public type: string, public eventInitDict: EventInit = {}) {
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

        composedPath(): EventTarget[] {
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

        initEvent(
            type: string,
            bubbles: boolean | undefined = undefined,
            cancelable: boolean | undefined = undefined
        ): void {
            this.type = type as string;
            this.bubbles = bubbles ?? false;
            this.cancelable = cancelable ?? false;
        }
    };
}

if (typeof globalThis.EventTarget !== "function") {
    // @ts-ignore
    globalThis.EventTarget = class EventTarget {
        private listeners: {
            [type: string]: { callback: (event: Event) => void, once: boolean; }[];
        } = {};

        addEventListener<T extends Event>(
            type: string,
            callback: (event: T) => void,
            options: { once?: boolean; } = {}
        ) {
            if (!(type in this.listeners)) {
                this.listeners[type] = [];
            }

            // @ts-ignore
            this.listeners[type].push({ callback, once: options?.once ?? false });
        }

        removeEventListener<T extends Event>(type: string, callback: (event: T) => void) {
            if (!(type in this.listeners)) {
                return;
            }

            const stack = this.listeners[type]!;
            for (let i = 0, l = stack.length; i < l; i++) {
                if (stack[i]!.callback === callback) {
                    stack.splice(i, 1);
                    return;
                }
            }

            if (stack.length === 0) {
                delete this.listeners[type];
            }
        }

        dispatchEvent<T extends Event>(event: T): boolean {
            if (!(event.type in this.listeners)) {
                return true;
            }

            Object.defineProperties(event, {
                currentTarget: { configurable: true, value: this },
                target: { configurable: true, value: this },
            });

            const stack = this.listeners[event.type]!.slice();
            for (let i = 0, l = stack.length; i < l; i++) {
                const listener = stack[i]!;

                try {
                    listener.callback.call(this, event);
                } catch (err) {
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
