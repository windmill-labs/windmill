/**
 * y-websocket protocol helpers shared by the multiplayer server tests:
 * building the frames a real client sends, reading back what the server
 * answers, and a socket wrapper that records both.
 */

import { WebSocket } from 'ws'
import * as Y from 'yjs'
import * as syncProtocol from 'y-protocols/sync'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'

// Message types, as server.mjs numbers them.
export const messageSync = 0
export const messageAwareness = 1

// y-protocols/sync sub-types. These overlap numerically with the message types
// above (`syncStep1 === messageSync === 0`), so no runtime check can catch a
// caller that passes the wrong family: `hasSyncType` takes only these three.
export const syncStep1 = 0
export const syncStep2 = 1
export const syncUpdate = 2

/** `messageSync` + sync step 1 for an empty document, as y-websocket sends on open. */
export function syncStep1Message() {
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, messageSync)
  syncProtocol.writeSyncStep1(encoder, new Y.Doc())
  return encoding.toUint8Array(encoder)
}

/** `messageSync` + a Yjs update inserting `text` into the 'content' text type. */
export function updateMessage(text) {
  const doc = new Y.Doc()
  doc.getText('content').insert(0, text)
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, messageSync)
  syncProtocol.writeUpdate(encoder, Y.encodeStateAsUpdate(doc))
  return encoding.toUint8Array(encoder)
}

/** Read the (messageType, syncType) pair that prefixes a sync message. */
export function messageKind(data) {
  const decoder = decoding.createDecoder(data)
  const messageType = decoding.readVarUint(decoder)
  if (messageType !== messageSync) return { messageType }
  return { messageType, syncType: decoding.readVarUint(decoder) }
}

/**
 * A live mirror of what `client` has been sent: one document that each call
 * brings up to date with the frames received since the last one. Cheap enough
 * to call from a polling predicate, and never re-applies a frame.
 */
export function syncedDoc(client) {
  const doc = new Y.Doc()
  let applied = 0
  return () => {
    while (applied < client.received.length) {
      const data = client.received[applied++]
      if (messageKind(data).messageType !== messageSync) continue
      const decoder = decoding.createDecoder(data)
      decoding.readVarUint(decoder) // messageSync
      syncProtocol.readSyncMessage(decoder, encoding.createEncoder(), doc, null)
    }
    return doc
  }
}

/** Open a socket and collect every frame it receives, plus its close code. */
export function openClient(url, { onOpen } = {}) {
  const ws = new WebSocket(url)
  const received = []
  const client = { ws, received, closeCode: undefined, lastError: undefined }
  // Under the default binaryType 'nodebuffer' `ws` hands every frame over as a
  // Buffer, text frames included, so this is lossless for both.
  ws.on('message', (data) => received.push(new Uint8Array(data)))
  ws.on('close', (code) => {
    client.closeCode = code
  })
  // Keep the error rather than only silencing it: without a listener `ws` would
  // throw, and without the record a failed connection shows up as a `waitFor`
  // timeout with no cause attached.
  ws.on('error', (error) => {
    client.lastError = error
  })
  if (onOpen) ws.on('open', () => onOpen(ws))
  return client
}

/** Has `client` received a sync message of this sub-type (never a message type)? */
export function hasSyncType(client, syncType) {
  return client.received.some((data) => messageKind(data).syncType === syncType)
}
