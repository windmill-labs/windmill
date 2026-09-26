/**
 * Regression tests for the cold-start window of the multiplayer server.
 *
 * On a fresh process the first `wss.on('connection')` handler awaits the JWKS
 * fetch before `setupWSConnection` attaches a 'message' listener. A y-websocket
 * client sends sync step 1 the instant the socket opens, so any message landing
 * inside that window used to be dropped by `ws` and never answered.
 *
 * The fake JWKS endpoint below parks the response until the test releases it,
 * so the server provably has no key while the client is sending and the race is
 * reproduced on every run rather than on a lucky schedule.
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import crypto from 'node:crypto'
import { setTimeout as delay } from 'node:timers/promises'
import * as Y from 'yjs'
import * as syncProtocol from 'y-protocols/sync'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'

import { mintToken, startJwksServer, startMultiplayerServer, waitFor } from './helpers.mjs'
import {
  hasSyncType,
  messageKind,
  openClient,
  syncStep1Message,
  syncStep2,
  syncUpdate,
  updateMessage
} from './protocol.mjs'

const WORKSPACE = 'test_workspace'
const DOC_PATH = `${WORKSPACE}/f/foo/bar`
// Grace for frames the client has already written to the socket to be delivered
// over loopback and read by the (otherwise idle) server, before the key is
// released. Only these already-written bytes have to land in this time.
const FLIGHT_MARGIN_MS = 250
// Logged by server.mjs once the JWKS fetch resolves. While it is absent the
// server demonstrably has no key, so anything it receives is in the pre-auth window.
const KEY_LOADED = 'Successfully loaded Ed25519 public key'

test('cold start: a sync step 1 sent while the key is still being fetched is answered', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ hold: true })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  // The server has asked for the key and is parked on the reply, so it cannot
  // authenticate anyone until this test lets it.
  await waitFor(() => jwks.requests >= 1, { message: 'the server to request the JWKS' })
  assert.ok(!server.output.includes(KEY_LOADED))

  // Cold client: sends sync step 1 and an update the instant the socket opens,
  // i.e. while the server is still awaiting the JWKS response.
  let framesWritten = 0
  const cold = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => {
      ws.send(syncStep1Message(), () => framesWritten++)
      ws.send(updateMessage('hello'), () => framesWritten++)
    }
  })

  await waitFor(() => framesWritten === 2, { message: 'the cold client to write both frames' })
  // Both frames are on the wire while the server still has no key, so they can
  // only reach it inside the pre-auth window.
  assert.ok(!server.output.includes(KEY_LOADED))
  await delay(FLIGHT_MARGIN_MS)
  jwks.release()

  await waitFor(() => hasSyncType(cold, syncStep2), {
    message: 'the server to answer the cold client with sync step 2'
  })
  // The echo of our own update proves it was applied to the server-side doc.
  await waitFor(() => hasSyncType(cold, syncUpdate), {
    message: 'the server to broadcast back the update sent during the cold window'
  })
  cold.ws.close()

  // A second client must now see the update the cold client sent.
  const warm = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })
  t.after(() => warm.ws.close())

  await waitFor(() => hasSyncType(warm, syncStep2), {
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
  const jwks = await startJwksServer({ hold: true })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const { privateKey: otherKey } = crypto.generateKeyPairSync('ed25519')
  const forged = mintToken(otherKey, { workspaceId: WORKSPACE })

  let framesWritten = 0
  const client = openClient(`${server.url}/${DOC_PATH}?token=${forged}`, {
    onOpen: (ws) => ws.send(syncStep1Message(), () => framesWritten++)
  })

  // Buffer the frame first, then let verification run and reject.
  await waitFor(() => framesWritten === 1, { message: 'the forged client to write its frame' })
  await delay(FLIGHT_MARGIN_MS)
  jwks.release()

  await waitFor(() => client.closeCode !== undefined, { message: 'the forged connection to be closed' })
  assert.equal(client.closeCode, 4403)
  assert.deepEqual(client.received, [])
})

test('cold start: a peer that floods before authenticating is closed and the server keeps serving', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ hold: true })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  // Four 600 KiB frames sent while the key is still parked: the second one takes
  // the buffer past MAX_PREAUTH_BYTES (1 MiB). The token is valid and the key is
  // not released until after the close, so only the cap can close this socket.
  const flooder = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => {
      for (let i = 0; i < 4; i++) ws.send(Buffer.alloc(600 * 1024))
    }
  })

  await waitFor(() => flooder.closeCode !== undefined, { message: 'the flooding connection to be closed' })
  assert.equal(flooder.closeCode, 1009)
  assert.deepEqual(flooder.received, [])
  assert.ok(!server.output.includes(KEY_LOADED))

  // The server is unharmed and still syncs a well-behaved client.
  jwks.release()
  const healthy = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })
  t.after(() => healthy.ws.close())

  await waitFor(() => hasSyncType(healthy, syncStep2), {
    message: 'the server to still answer a well-behaved client with sync step 2'
  })
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
