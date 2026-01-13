export const messageSync: 0;
export const messageQueryAwareness: 3;
export const messageAwareness: 1;
export const messageAuth: 2;
/**
 * Websocket Provider for Yjs. Creates a websocket connection to sync the shared document.
 * The document name is attached to the provided url. I.e. the following example
 * creates a websocket connection to http://localhost:1234/my-document-name
 *
 * @example
 *   import * as Y from 'yjs'
 *   import { WebsocketProvider } from 'y-websocket'
 *   const doc = new Y.Doc()
 *   const provider = new WebsocketProvider('http://localhost:1234', 'my-document-name', doc)
 *
 * @extends {ObservableV2<{ 'connection-close': (event: CloseEvent | null,  provider: WebsocketProvider) => any, 'status': (event: { status: 'connected' | 'disconnected' | 'connecting' }) => any, 'connection-error': (event: Event, provider: WebsocketProvider) => any, 'sync': (state: boolean) => any }>}
 */
export class WebsocketProvider extends ObservableV2<{
    'connection-close': (event: CloseEvent | null, provider: WebsocketProvider) => any;
    status: (event: {
        status: 'connected' | 'disconnected' | 'connecting';
    }) => any;
    'connection-error': (event: Event, provider: WebsocketProvider) => any;
    sync: (state: boolean) => any;
}> {
    /**
     * @param {string} serverUrl
     * @param {string} roomname
     * @param {Y.Doc} doc
     * @param {object} opts
     * @param {boolean} [opts.connect]
     * @param {awarenessProtocol.Awareness} [opts.awareness]
     * @param {Object<string,string>} [opts.params] specify url parameters
     * @param {Array<string>} [opts.protocols] specify websocket protocols
     * @param {typeof WebSocket} [opts.WebSocketPolyfill] Optionall provide a WebSocket polyfill
     * @param {number} [opts.resyncInterval] Request server state every `resyncInterval` milliseconds
     * @param {number} [opts.maxBackoffTime] Maximum amount of time to wait before trying to reconnect (we try to reconnect using exponential backoff)
     * @param {boolean} [opts.disableBc] Disable cross-tab BroadcastChannel communication
     */
    constructor(serverUrl: string, roomname: string, doc: Y.Doc, { connect, awareness, params, protocols, WebSocketPolyfill, resyncInterval, maxBackoffTime, disableBc }?: {
        connect?: boolean | undefined;
        awareness?: awarenessProtocol.Awareness | undefined;
        params?: {
            [x: string]: string;
        } | undefined;
        protocols?: string[] | undefined;
        WebSocketPolyfill?: {
            new (url: string | URL, protocols?: string | string[] | undefined): WebSocket;
            prototype: WebSocket;
            readonly CLOSED: number;
            readonly CLOSING: number;
            readonly CONNECTING: number;
            readonly OPEN: number;
        } | undefined;
        resyncInterval?: number | undefined;
        maxBackoffTime?: number | undefined;
        disableBc?: boolean | undefined;
    });
    serverUrl: string;
    bcChannel: string;
    maxBackoffTime: number;
    /**
     * The specified url parameters. This can be safely updated. The changed parameters will be used
     * when a new connection is established.
     * @type {Object<string,string>}
     */
    params: {
        [x: string]: string;
    };
    protocols: string[];
    roomname: string;
    doc: Y.Doc;
    _WS: {
        new (url: string | URL, protocols?: string | string[] | undefined): WebSocket;
        prototype: WebSocket;
        readonly CLOSED: number;
        readonly CLOSING: number;
        readonly CONNECTING: number;
        readonly OPEN: number;
    };
    awareness: awarenessProtocol.Awareness;
    wsconnected: boolean;
    wsconnecting: boolean;
    bcconnected: boolean;
    disableBc: boolean;
    wsUnsuccessfulReconnects: number;
    messageHandlers: ((arg0: encoding.Encoder, arg1: decoding.Decoder, arg2: WebsocketProvider, arg3: boolean, arg4: number) => void)[];
    /**
     * @type {boolean}
     */
    _synced: boolean;
    /**
     * @type {WebSocket?}
     */
    ws: WebSocket | null;
    wsLastMessageReceived: number;
    /**
     * Whether to connect to other peers or not
     * @type {boolean}
     */
    shouldConnect: boolean;
    /**
     * @type {number}
     */
    _resyncInterval: number;
    /**
     * @param {ArrayBuffer} data
     * @param {any} origin
     */
    _bcSubscriber: (data: ArrayBuffer, origin: any) => void;
    /**
     * Listens to Yjs updates and sends them to remote peers (ws and broadcastchannel)
     * @param {Uint8Array} update
     * @param {any} origin
     */
    _updateHandler: (update: Uint8Array, origin: any) => void;
    /**
     * @param {any} changed
     * @param {any} _origin
     */
    _awarenessUpdateHandler: ({ added, updated, removed }: any, _origin: any) => void;
    _exitHandler: () => void;
    _checkInterval: any;
    get url(): string;
    set synced(arg: boolean);
    /**
     * @type {boolean}
     */
    get synced(): boolean;
    connectBc(): void;
    disconnectBc(): void;
    disconnect(): void;
    connect(): void;
}
import { ObservableV2 } from "lib0/observable";
import * as Y from "yjs";
import * as awarenessProtocol from "y-protocols/awareness";
import * as encoding from "lib0/encoding";
import * as decoding from "lib0/decoding";
