/**
 * Regression tests for the cold-start window of the multiplayer server.
 *
 * On a fresh process the first `wss.on('connection')` handler awaits the JWKS
 * fetch before `setupWSConnection` attaches a 'message' listener. A y-websocket
 * client sends sync step 1 the instant the socket opens, so any message landing
 * inside that window used to be dropped by `ws` and never answered.
 *
 * The fake JWKS endpoint below answers with a delay, which keeps that window
 * open for a known duration and makes the race deterministic.
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import crypto from 'node:crypto'
import { WebSocket } from 'ws'
import * as Y from 'yjs'
import * as syncProtocol from 'y-protocols/sync'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'

import { mintToken, startJwksServer, startMultiplayerServer, waitFor } from './helpers.mjs'

const WORKSPACE = 'test_workspace'
const DOC_PATH = `${WORKSPACE}/f/foo/bar`
// Long enough that the JWKS fetch is guaranteed to still be in flight when the
// client connects, on any machine, including with the startup prefetch.
const JWKS_DELAY_MS = 1500

const messageSync = 0
const syncStep2 = 1

/** `messageSync` + sync step 1 for an empty document, as y-websocket sends on open. */
function syncStep1Message() {
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, messageSync)
  syncProtocol.writeSyncStep1(encoder, new Y.Doc())
  return encoding.toUint8Array(encoder)
}

/** `messageSync` + a Yjs update inserting `text` into the 'content' text type. */
function updateMessage(text) {
  const doc = new Y.Doc()
  doc.getText('content').insert(0, text)
  const encoder = encoding.createEncoder()
  encoding.writeVarUint(encoder, messageSync)
  syncProtocol.writeUpdate(encoder, Y.encodeStateAsUpdate(doc))
  return encoding.toUint8Array(encoder)
}

/** Read the (messageType, syncType) pair that prefixes a sync message. */
function messageKind(data) {
  const decoder = decoding.createDecoder(data)
  const messageType = decoding.readVarUint(decoder)
  if (messageType !== messageSync) return { messageType }
  return { messageType, syncType: decoding.readVarUint(decoder) }
}

/** Open a socket and collect every frame it receives, plus its close code. */
function openClient(url, { onOpen } = {}) {
  const ws = new WebSocket(url)
  const received = []
  const client = { ws, received, closeCode: undefined }
  ws.on('message', (data) => received.push(new Uint8Array(data)))
  ws.on('close', (code) => {
    client.closeCode = code
  })
  ws.on('error', () => {})
  if (onOpen) ws.on('open', () => onOpen(ws))
  return client
}

function hasKind(client, syncType) {
  return client.received.some((data) => messageKind(data).syncType === syncType)
}

test('cold start: a sync step 1 sent while the key is still being fetched is answered', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ delayMs: JWKS_DELAY_MS })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  // Cold client: sends sync step 1 and an update the instant the socket opens,
  // i.e. while the server is still awaiting the JWKS response.
  const cold = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => {
      ws.send(syncStep1Message())
      ws.send(updateMessage('hello'))
    }
  })

  await waitFor(() => hasKind(cold, syncStep2), {
    message: 'the server to answer the cold client with sync step 2'
  })
  // The echo of our own update proves it was applied to the server-side doc.
  await waitFor(() => hasKind(cold, 2), {
    message: 'the server to broadcast back the update sent during the cold window'
  })
  cold.ws.close()

  // A second client must now see the update the cold client sent.
  const warm = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })
  t.after(() => warm.ws.close())

  await waitFor(() => hasKind(warm, syncStep2), {
    message: 'the server to answer the second client with sync step 2'
  })

  const doc = new Y.Doc()
  const step2 = warm.received.find((data) => messageKind(data).syncType === syncStep2)
  const decoder = decoding.createDecoder(step2)
  decoding.readVarUint(decoder) // messageSync
  syncProtocol.readSyncMessage(decoder, encoding.createEncoder(), doc, null)
  assert.equal(doc.getText('content').toString(), 'hello')
})

test('cold start: a forged token is rejected with 4403 and its messages are dropped', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ delayMs: JWKS_DELAY_MS })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const { privateKey: otherKey } = crypto.generateKeyPairSync('ed25519')
  const forged = mintToken(otherKey, { workspaceId: WORKSPACE })

  const client = openClient(`${server.url}/${DOC_PATH}?token=${forged}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })

  await waitFor(() => client.closeCode !== undefined, { message: 'the forged connection to be closed' })
  assert.equal(client.closeCode, 4403)
  assert.deepEqual(client.received, [])
})

test('a connection without a token is rejected with 4401', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer()
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const client = openClient(`${server.url}/${DOC_PATH}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })

  await waitFor(() => client.closeCode !== undefined, { message: 'the unauthenticated connection to be closed' })
  assert.equal(client.closeCode, 4401)
  assert.deepEqual(client.received, [])
})

test('the public key is fetched at startup, before any client connects', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer()
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  await waitFor(() => jwks.requests >= 1, { message: 'the server to prefetch the JWKS at startup' })
})
