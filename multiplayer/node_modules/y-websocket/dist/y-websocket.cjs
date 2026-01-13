'use strict';

require('yjs');
var bc = require('lib0/broadcastchannel');
var time = require('lib0/time');
var encoding = require('lib0/encoding');
var decoding = require('lib0/decoding');
var syncProtocol = require('y-protocols/sync');
var authProtocol = require('y-protocols/auth');
var awarenessProtocol = require('y-protocols/awareness');
var observable = require('lib0/observable');
var math = require('lib0/math');
var url = require('lib0/url');
var env = require('lib0/environment');

function _interopNamespaceDefault(e) {
  var n = Object.create(null);
  if (e) {
    Object.keys(e).forEach(function (k) {
      if (k !== 'default') {
        var d = Object.getOwnPropertyDescriptor(e, k);
        Object.defineProperty(n, k, d.get ? d : {
          enumerable: true,
          get: function () { return e[k]; }
        });
      }
    });
  }
  n.default = e;
  return Object.freeze(n);
}

var bc__namespace = /*#__PURE__*/_interopNamespaceDefault(bc);
var time__namespace = /*#__PURE__*/_interopNamespaceDefault(time);
var encoding__namespace = /*#__PURE__*/_interopNamespaceDefault(encoding);
var decoding__namespace = /*#__PURE__*/_interopNamespaceDefault(decoding);
var syncProtocol__namespace = /*#__PURE__*/_interopNamespaceDefault(syncProtocol);
var authProtocol__namespace = /*#__PURE__*/_interopNamespaceDefault(authProtocol);
var awarenessProtocol__namespace = /*#__PURE__*/_interopNamespaceDefault(awarenessProtocol);
var math__namespace = /*#__PURE__*/_interopNamespaceDefault(math);
var url__namespace = /*#__PURE__*/_interopNamespaceDefault(url);
var env__namespace = /*#__PURE__*/_interopNamespaceDefault(env);

/**
 * @module provider/websocket
 */


const messageSync = 0;
const messageQueryAwareness = 3;
const messageAwareness = 1;
const messageAuth = 2;

/**
 *                       encoder,          decoder,          provider,          emitSynced, messageType
 * @type {Array<function(encoding.Encoder, decoding.Decoder, WebsocketProvider, boolean,    number):void>}
 */
const messageHandlers = [];

messageHandlers[messageSync] = (
  encoder,
  decoder,
  provider,
  emitSynced,
  _messageType
) => {
  encoding__namespace.writeVarUint(encoder, messageSync);
  const syncMessageType = syncProtocol__namespace.readSyncMessage(
    decoder,
    encoder,
    provider.doc,
    provider
  );
  if (
    emitSynced && syncMessageType === syncProtocol__namespace.messageYjsSyncStep2 &&
    !provider.synced
  ) {
    provider.synced = true;
  }
};

messageHandlers[messageQueryAwareness] = (
  encoder,
  _decoder,
  provider,
  _emitSynced,
  _messageType
) => {
  encoding__namespace.writeVarUint(encoder, messageAwareness);
  encoding__namespace.writeVarUint8Array(
    encoder,
    awarenessProtocol__namespace.encodeAwarenessUpdate(
      provider.awareness,
      Array.from(provider.awareness.getStates().keys())
    )
  );
};

messageHandlers[messageAwareness] = (
  _encoder,
  decoder,
  provider,
  _emitSynced,
  _messageType
) => {
  awarenessProtocol__namespace.applyAwarenessUpdate(
    provider.awareness,
    decoding__namespace.readVarUint8Array(decoder),
    provider
  );
};

messageHandlers[messageAuth] = (
  _encoder,
  decoder,
  provider,
  _emitSynced,
  _messageType
) => {
  authProtocol__namespace.readAuthMessage(
    decoder,
    provider.doc,
    (_ydoc, reason) => permissionDeniedHandler(provider, reason)
  );
};

// @todo - this should depend on awareness.outdatedTime
const messageReconnectTimeout = 30000;

/**
 * @param {WebsocketProvider} provider
 * @param {string} reason
 */
const permissionDeniedHandler = (provider, reason) =>
  console.warn(`Permission denied to access ${provider.url}.\n${reason}`);

/**
 * @param {WebsocketProvider} provider
 * @param {Uint8Array} buf
 * @param {boolean} emitSynced
 * @return {encoding.Encoder}
 */
const readMessage = (provider, buf, emitSynced) => {
  const decoder = decoding__namespace.createDecoder(buf);
  const encoder = encoding__namespace.createEncoder();
  const messageType = decoding__namespace.readVarUint(decoder);
  const messageHandler = provider.messageHandlers[messageType];
  if (/** @type {any} */ (messageHandler)) {
    messageHandler(encoder, decoder, provider, emitSynced, messageType);
  } else {
    console.error('Unable to compute message');
  }
  return encoder
};

/**
 * Outsource this function so that a new websocket connection is created immediately.
 * I suspect that the `ws.onclose` event is not always fired if there are network issues.
 *
 * @param {WebsocketProvider} provider
 * @param {WebSocket} ws
 * @param {CloseEvent | null} event
 */
const closeWebsocketConnection = (provider, ws, event) => {
  if (ws === provider.ws) {
    provider.emit('connection-close', [event, provider]);
    provider.ws = null;
    ws.close();
    provider.wsconnecting = false;
    if (provider.wsconnected) {
      provider.wsconnected = false;
      provider.synced = false;
      // update awareness (all users except local left)
      awarenessProtocol__namespace.removeAwarenessStates(
        provider.awareness,
        Array.from(provider.awareness.getStates().keys()).filter((client) =>
          client !== provider.doc.clientID
        ),
        provider
      );
      provider.emit('status', [{
        status: 'disconnected'
      }]);
    } else {
      provider.wsUnsuccessfulReconnects++;
    }
    // Start with no reconnect timeout and increase timeout by
    // using exponential backoff starting with 100ms
    setTimeout(
      setupWS,
      math__namespace.min(
        math__namespace.pow(2, provider.wsUnsuccessfulReconnects) * 100,
        provider.maxBackoffTime
      ),
      provider
    );
  }
};

/**
 * @param {WebsocketProvider} provider
 */
const setupWS = (provider) => {
  if (provider.shouldConnect && provider.ws === null) {
    const websocket = new provider._WS(provider.url, provider.protocols);
    websocket.binaryType = 'arraybuffer';
    provider.ws = websocket;
    provider.wsconnecting = true;
    provider.wsconnected = false;
    provider.synced = false;

    websocket.onmessage = (event) => {
      provider.wsLastMessageReceived = time__namespace.getUnixTime();
      const encoder = readMessage(provider, new Uint8Array(event.data), true);
      if (encoding__namespace.length(encoder) > 1) {
        websocket.send(encoding__namespace.toUint8Array(encoder));
      }
    };
    websocket.onerror = (event) => {
      provider.emit('connection-error', [event, provider]);
    };
    websocket.onclose = (event) => {
      closeWebsocketConnection(provider, websocket, event);
    };
    websocket.onopen = () => {
      provider.wsLastMessageReceived = time__namespace.getUnixTime();
      provider.wsconnecting = false;
      provider.wsconnected = true;
      provider.wsUnsuccessfulReconnects = 0;
      provider.emit('status', [{
        status: 'connected'
      }]);
      // always send sync step 1 when connected
      const encoder = encoding__namespace.createEncoder();
      encoding__namespace.writeVarUint(encoder, messageSync);
      syncProtocol__namespace.writeSyncStep1(encoder, provider.doc);
      websocket.send(encoding__namespace.toUint8Array(encoder));
      // broadcast local awareness state
      if (provider.awareness.getLocalState() !== null) {
        const encoderAwarenessState = encoding__namespace.createEncoder();
        encoding__namespace.writeVarUint(encoderAwarenessState, messageAwareness);
        encoding__namespace.writeVarUint8Array(
          encoderAwarenessState,
          awarenessProtocol__namespace.encodeAwarenessUpdate(provider.awareness, [
            provider.doc.clientID
          ])
        );
        websocket.send(encoding__namespace.toUint8Array(encoderAwarenessState));
      }
    };
    provider.emit('status', [{
      status: 'connecting'
    }]);
  }
};

/**
 * @param {WebsocketProvider} provider
 * @param {ArrayBuffer} buf
 */
const broadcastMessage = (provider, buf) => {
  const ws = provider.ws;
  if (provider.wsconnected && ws && ws.readyState === ws.OPEN) {
    ws.send(buf);
  }
  if (provider.bcconnected) {
    bc__namespace.publish(provider.bcChannel, buf, provider);
  }
};

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
class WebsocketProvider extends observable.ObservableV2 {
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
  constructor (serverUrl, roomname, doc, {
    connect = true,
    awareness = new awarenessProtocol__namespace.Awareness(doc),
    params = {},
    protocols = [],
    WebSocketPolyfill = WebSocket,
    resyncInterval = -1,
    maxBackoffTime = 2500,
    disableBc = false
  } = {}) {
    super();
    // ensure that serverUrl does not end with /
    while (serverUrl[serverUrl.length - 1] === '/') {
      serverUrl = serverUrl.slice(0, serverUrl.length - 1);
    }
    this.serverUrl = serverUrl;
    this.bcChannel = serverUrl + '/' + roomname;
    this.maxBackoffTime = maxBackoffTime;
    /**
     * The specified url parameters. This can be safely updated. The changed parameters will be used
     * when a new connection is established.
     * @type {Object<string,string>}
     */
    this.params = params;
    this.protocols = protocols;
    this.roomname = roomname;
    this.doc = doc;
    this._WS = WebSocketPolyfill;
    this.awareness = awareness;
    this.wsconnected = false;
    this.wsconnecting = false;
    this.bcconnected = false;
    this.disableBc = disableBc;
    this.wsUnsuccessfulReconnects = 0;
    this.messageHandlers = messageHandlers.slice();
    /**
     * @type {boolean}
     */
    this._synced = false;
    /**
     * @type {WebSocket?}
     */
    this.ws = null;
    this.wsLastMessageReceived = 0;
    /**
     * Whether to connect to other peers or not
     * @type {boolean}
     */
    this.shouldConnect = connect;

    /**
     * @type {number}
     */
    this._resyncInterval = 0;
    if (resyncInterval > 0) {
      this._resyncInterval = /** @type {any} */ (setInterval(() => {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
          // resend sync step 1
          const encoder = encoding__namespace.createEncoder();
          encoding__namespace.writeVarUint(encoder, messageSync);
          syncProtocol__namespace.writeSyncStep1(encoder, doc);
          this.ws.send(encoding__namespace.toUint8Array(encoder));
        }
      }, resyncInterval));
    }

    /**
     * @param {ArrayBuffer} data
     * @param {any} origin
     */
    this._bcSubscriber = (data, origin) => {
      if (origin !== this) {
        const encoder = readMessage(this, new Uint8Array(data), false);
        if (encoding__namespace.length(encoder) > 1) {
          bc__namespace.publish(this.bcChannel, encoding__namespace.toUint8Array(encoder), this);
        }
      }
    };
    /**
     * Listens to Yjs updates and sends them to remote peers (ws and broadcastchannel)
     * @param {Uint8Array} update
     * @param {any} origin
     */
    this._updateHandler = (update, origin) => {
      if (origin !== this) {
        const encoder = encoding__namespace.createEncoder();
        encoding__namespace.writeVarUint(encoder, messageSync);
        syncProtocol__namespace.writeUpdate(encoder, update);
        broadcastMessage(this, encoding__namespace.toUint8Array(encoder));
      }
    };
    this.doc.on('update', this._updateHandler);
    /**
     * @param {any} changed
     * @param {any} _origin
     */
    this._awarenessUpdateHandler = ({ added, updated, removed }, _origin) => {
      const changedClients = added.concat(updated).concat(removed);
      const encoder = encoding__namespace.createEncoder();
      encoding__namespace.writeVarUint(encoder, messageAwareness);
      encoding__namespace.writeVarUint8Array(
        encoder,
        awarenessProtocol__namespace.encodeAwarenessUpdate(awareness, changedClients)
      );
      broadcastMessage(this, encoding__namespace.toUint8Array(encoder));
    };
    this._exitHandler = () => {
      awarenessProtocol__namespace.removeAwarenessStates(
        this.awareness,
        [doc.clientID],
        'app closed'
      );
    };
    if (env__namespace.isNode && typeof process !== 'undefined') {
      process.on('exit', this._exitHandler);
    }
    awareness.on('update', this._awarenessUpdateHandler);
    this._checkInterval = /** @type {any} */ (setInterval(() => {
      if (
        this.wsconnected &&
        messageReconnectTimeout <
          time__namespace.getUnixTime() - this.wsLastMessageReceived
      ) {
        // no message received in a long time - not even your own awareness
        // updates (which are updated every 15 seconds)
        closeWebsocketConnection(this, /** @type {WebSocket} */ (this.ws), null);
      }
    }, messageReconnectTimeout / 10));
    if (connect) {
      this.connect();
    }
  }

  get url () {
    const encodedParams = url__namespace.encodeQueryParams(this.params);
    return this.serverUrl + '/' + this.roomname +
      (encodedParams.length === 0 ? '' : '?' + encodedParams)
  }

  /**
   * @type {boolean}
   */
  get synced () {
    return this._synced
  }

  set synced (state) {
    if (this._synced !== state) {
      this._synced = state;
      // @ts-ignore
      this.emit('synced', [state]);
      this.emit('sync', [state]);
    }
  }

  destroy () {
    if (this._resyncInterval !== 0) {
      clearInterval(this._resyncInterval);
    }
    clearInterval(this._checkInterval);
    this.disconnect();
    if (env__namespace.isNode && typeof process !== 'undefined') {
      process.off('exit', this._exitHandler);
    }
    this.awareness.off('update', this._awarenessUpdateHandler);
    this.doc.off('update', this._updateHandler);
    super.destroy();
  }

  connectBc () {
    if (this.disableBc) {
      return
    }
    if (!this.bcconnected) {
      bc__namespace.subscribe(this.bcChannel, this._bcSubscriber);
      this.bcconnected = true;
    }
    // send sync step1 to bc
    // write sync step 1
    const encoderSync = encoding__namespace.createEncoder();
    encoding__namespace.writeVarUint(encoderSync, messageSync);
    syncProtocol__namespace.writeSyncStep1(encoderSync, this.doc);
    bc__namespace.publish(this.bcChannel, encoding__namespace.toUint8Array(encoderSync), this);
    // broadcast local state
    const encoderState = encoding__namespace.createEncoder();
    encoding__namespace.writeVarUint(encoderState, messageSync);
    syncProtocol__namespace.writeSyncStep2(encoderState, this.doc);
    bc__namespace.publish(this.bcChannel, encoding__namespace.toUint8Array(encoderState), this);
    // write queryAwareness
    const encoderAwarenessQuery = encoding__namespace.createEncoder();
    encoding__namespace.writeVarUint(encoderAwarenessQuery, messageQueryAwareness);
    bc__namespace.publish(
      this.bcChannel,
      encoding__namespace.toUint8Array(encoderAwarenessQuery),
      this
    );
    // broadcast local awareness state
    const encoderAwarenessState = encoding__namespace.createEncoder();
    encoding__namespace.writeVarUint(encoderAwarenessState, messageAwareness);
    encoding__namespace.writeVarUint8Array(
      encoderAwarenessState,
      awarenessProtocol__namespace.encodeAwarenessUpdate(this.awareness, [
        this.doc.clientID
      ])
    );
    bc__namespace.publish(
      this.bcChannel,
      encoding__namespace.toUint8Array(encoderAwarenessState),
      this
    );
  }

  disconnectBc () {
    // broadcast message with local awareness state set to null (indicating disconnect)
    const encoder = encoding__namespace.createEncoder();
    encoding__namespace.writeVarUint(encoder, messageAwareness);
    encoding__namespace.writeVarUint8Array(
      encoder,
      awarenessProtocol__namespace.encodeAwarenessUpdate(this.awareness, [
        this.doc.clientID
      ], new Map())
    );
    broadcastMessage(this, encoding__namespace.toUint8Array(encoder));
    if (this.bcconnected) {
      bc__namespace.unsubscribe(this.bcChannel, this._bcSubscriber);
      this.bcconnected = false;
    }
  }

  disconnect () {
    this.shouldConnect = false;
    this.disconnectBc();
    if (this.ws !== null) {
      closeWebsocketConnection(this, this.ws, null);
    }
  }

  connect () {
    this.shouldConnect = true;
    if (!this.wsconnected && this.ws === null) {
      setupWS(this);
      this.connectBc();
    }
  }
}

exports.WebsocketProvider = WebsocketProvider;
exports.messageAuth = messageAuth;
exports.messageAwareness = messageAwareness;
exports.messageQueryAwareness = messageQueryAwareness;
exports.messageSync = messageSync;
//# sourceMappingURL=y-websocket.cjs.map
