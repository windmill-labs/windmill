import "../external/event-target-polyfill/index.ts";
import chan from "../chan.ts";
import { fromErrorEvent } from "../error.ts";
import type { Ensured } from "../types.ts";
import type { WebSocketServer } from "../ws.ts";

const _source = Symbol.for("source");
const _ws = Symbol.for("ws");

export type WebSocketLike = Ensured<Partial<WebSocket>, "readyState" | "close" | "send">;

/**
 * This class represents a WebSocket connection on the server side.
 * Normally we don't create instances of this class directly, but rather use
 * the {@link WebSocketServer} to handle WebSocket connections, which will
 * create the instance for us.
 * 
 * **Events:**
 * 
 * - `open` - Dispatched when the connection is ready.
 * - `message` - Dispatched when a message is received.
 * - `error` - Dispatched when an error occurs, such as network failure. After
 *   this event is dispatched, the connection will be closed and the `close`
 *   event will be dispatched.
 * - `close` - Dispatched when the connection is closed. If the connection is
 *   closed due to some error, the `error` event will be dispatched before this
 *   event, and the close event will have the `wasClean` set to `false`, and the
 *   `reason` property contains the error message, if any.
 */
export class WebSocketConnection extends EventTarget implements AsyncIterable<string | Uint8Array> {
    private [_source]: Promise<WebSocketLike>;
    private [_ws]: WebSocketLike | null = null;

    constructor(source: Promise<WebSocketLike>) {
        super();
        this[_source] = source;
        this[_source].then(ws => {
            this[_ws] = ws;
        });
    }

    /**
     * A promise that resolves when the connection is ready to send and receive
     * messages.
     * 
     * @deprecated Listen for the `open` event instead.
     */
    get ready(): Promise<this> {
        return this[_source].then(() => this);
    }

    /**
     * The current state of the WebSocket connection.
     */
    get readyState(): number {
        return this[_ws]?.readyState ?? 0;
    }

    /**
     * Sends data to the WebSocket client.
     */
    send(data: string | ArrayBufferLike | ArrayBufferView): void {
        if (!this[_ws]) {
            throw new Error("WebSocket connection is not ready");
        }

        this[_ws].send(data);
    }

    /**
     * Closes the WebSocket connection.
     */
    close(code?: number | undefined, reason?: string | undefined): void {
        if (!this[_ws]) {
            throw new Error("WebSocket connection is not ready");
        }

        this[_ws].close(code, reason);
    }

    /**
     * Adds an event listener that will be called when the connection is ready.
     */
    override addEventListener(
        type: "open",
        listener: (this: WebSocketConnection, ev: Event) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when a message is received.
     */
    override addEventListener(
        type: "message",
        listener: (this: WebSocketConnection, ev: MessageEvent<string | Uint8Array>) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when the connection is
     * interrupted. After this event is dispatched, the connection will be
     * closed and the `close` event will be dispatched.
     */
    override addEventListener(
        type: "error",
        listener: (this: WebSocketConnection, ev: ErrorEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    /**
     * Adds an event listener that will be called when the connection is closed.
     * If the connection is closed due to some error, the `error` event will be
     * dispatched before this event, and the close event will have the `wasClean`
     * set to `false`, and the `reason` property contains the error message, if
     * any.
     */
    override addEventListener(
        type: "close",
        listener: (this: WebSocketConnection, ev: CloseEvent) => void,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | AddEventListenerOptions
    ): void;
    override addEventListener(
        event: string,
        listener: any,
        options: boolean | AddEventListenerOptions | undefined = undefined
    ): void {
        return super.addEventListener(event, listener, options);
    }

    override removeEventListener(
        type: "open",
        listener: (this: WebSocketConnection, ev: Event) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "message",
        listener: (this: WebSocketConnection, ev: MessageEvent<string | Uint8Array>) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "error",
        listener: (this: WebSocketConnection, ev: ErrorEvent) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: "close",
        listener: (this: WebSocketConnection, ev: CloseEvent) => void,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        type: string,
        listener: EventListenerOrEventListenerObject | null,
        options?: boolean | EventListenerOptions
    ): void;
    override removeEventListener(
        event: string,
        listener: any,
        options: boolean | EventListenerOptions | undefined = undefined
    ): void {
        return super.removeEventListener(event, listener, options);
    }

    async *[Symbol.asyncIterator](): AsyncIterableIterator<string | Uint8Array> {
        const channel = chan<string | Uint8Array>(Infinity);
        const handleMessage = (ev: MessageEvent<string | Uint8Array>) => {
            channel.send(ev.data);
        };
        const handleClose = (ev: CloseEvent) => {
            ev.wasClean && channel.close();
        };
        const handleError = (ev: ErrorEvent) => {
            channel.close(fromErrorEvent(ev));
        };

        this.addEventListener("message", handleMessage);
        this.addEventListener("close", handleClose);
        this.addEventListener("error", handleError);

        try {
            for await (const data of channel) {
                yield data;
            }
        } finally {
            this.removeEventListener("message", handleMessage);
            this.removeEventListener("close", handleClose);
            this.removeEventListener("error", handleError);
        }
    }
}

/**
 * WebSocket handler function for the {@link WebSocketServer} constructor.
 */
export type WebSocketHandler = (socket: WebSocketConnection) => void;

/**
 * Options for the {@link WebSocketServer} constructor.
 */
export interface ServerOptions {
    /**
     * The idle timeout in seconds. The server will close the connection if no
     * messages are received within this time.
     * 
     * NOTE: Currently, this option is only supported in Deno and Bun, in other
     * environments, the option is ignored.
     */
    idleTimeout?: number;
    /**
     * Whether to enable per-message deflate compression.
     * 
     * NOTE: Currently, this option is only supported in Node.js and Bun, in
     * other environments, the option is ignored.
     */
    perMessageDeflate?: boolean;
}
