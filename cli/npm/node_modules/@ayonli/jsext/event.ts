/**
 * Functions for working with events.
 * @module
 */
import "./external/event-target-polyfill/index.ts";

/**
 * Creates an `ErrorEvent` instance based on the given options. If the
 * `ErrorEvent` constructor is not available, the generic `Event` constructor
 * will be used instead, and the options will be attached to the event as its
 * properties.
 */
export function createErrorEvent(type: "error", options?: ErrorEventInit): ErrorEvent;
export function createErrorEvent(type: string, options?: ErrorEventInit): ErrorEvent;
export function createErrorEvent(type: string, options: ErrorEventInit = {}): ErrorEvent {
    if (typeof ErrorEvent === "function") {
        return new ErrorEvent(type, options);
    } else {
        const event = new Event(type, {
            bubbles: options?.bubbles ?? false,
            cancelable: options?.cancelable ?? false,
            composed: options?.composed ?? false,
        });

        Object.defineProperties(event, {
            message: { configurable: true, value: options?.message ?? "" },
            filename: { configurable: true, value: options?.filename ?? "" },
            lineno: { configurable: true, value: options?.lineno ?? 0 },
            colno: { configurable: true, value: options?.colno ?? 0 },
            error: { configurable: true, value: options?.error ?? undefined },
        });

        return event as ErrorEvent;
    }
}

/**
 * Creates a `CloseEvent` instance based on the given options. If the
 * `CloseEvent` constructor is not available, the generic `Event` constructor
 * will be used instead, and the options will be attached to the event as its
 * properties.
 */
export function createCloseEvent(type: "close", options?: CloseEventInit): CloseEvent;
export function createCloseEvent(type: string, options?: CloseEventInit): CloseEvent;
export function createCloseEvent(type: string, options: CloseEventInit = {}): CloseEvent {
    if (typeof CloseEvent === "function") {
        return new CloseEvent(type, options);
    } else {
        const event = new Event(type, {
            bubbles: options?.bubbles ?? false,
            cancelable: options?.cancelable ?? false,
            composed: options?.composed ?? false,
        });

        Object.defineProperties(event, {
            code: { configurable: true, value: options.code ?? 0 },
            reason: { configurable: true, value: options.reason ?? "" },
            wasClean: { configurable: true, value: options.wasClean ?? false },
        });

        return event as CloseEvent;
    }
}

/**
 * Creates a `ProgressEvent` instance based on the given options. If the
 * `ProgressEvent` constructor is not available, the generic `Event` constructor
 * will be used instead, and the options will be attached to the event as its
 * properties.
 */
export function createProgressEvent(type: "progress", options?: ProgressEventInit): ProgressEvent;
export function createProgressEvent(type: string, options?: ProgressEventInit): ProgressEvent;
export function createProgressEvent(type: string, options: ProgressEventInit = {}): ProgressEvent {
    if (typeof ProgressEvent === "function") {
        return new ProgressEvent(type, options);
    } else {
        const event = new Event(type, {
            bubbles: options?.bubbles ?? false,
            cancelable: options?.cancelable ?? false,
            composed: options?.composed ?? false,
        });

        Object.defineProperties(event, {
            lengthComputable: { configurable: true, value: options?.lengthComputable ?? false },
            loaded: { configurable: true, value: options?.loaded ?? 0 },
            total: { configurable: true, value: options?.total ?? 0 },
        });

        return event as ProgressEvent;
    }
}

/**
 * Creates a `CustomEvent` instance based on the given options. If the
 * `CustomEvent` constructor is not available, the generic `Event` constructor
 * will be used instead, and the options will be attached to the event as its
 * properties.
 */
export function createCustomEvent<T = any>(
    type: string,
    options: CustomEventInit<T> = {}
): CustomEvent<T> {
    if (typeof CustomEvent === "function") {
        return new CustomEvent(type, options);
    } else {
        const event = new Event(type, {
            bubbles: options?.bubbles ?? false,
            cancelable: options?.cancelable ?? false,
            composed: options?.composed ?? false,
        });

        Object.defineProperties(event, {
            detail: { configurable: true, value: options?.detail ?? null },
        });

        return event as CustomEvent<T>;
    }
}
