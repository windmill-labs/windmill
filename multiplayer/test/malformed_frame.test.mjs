/**
 * Regression tests for malformed frames sent by an authenticated peer.
 *
 * The 'message' handler in `setupWSConnection` decodes whatever arrives with
 * `decoding.readVarUint`, `syncProtocol.readSyncMessage` and
 * `awarenessProtocol.applyAwarenessUpdate`. All three throw on input they cannot
 * parse, and `ws` emits the listener's exception on the process. With no
 * `uncaughtException` handler installed, one such frame used to take the whole
 * multiplayer server down — every other document and every other client with it.
 *
 * The payloads below are the ones that reproduce it: 600 KiB of zeros decodes as
 * sync step 1 with an empty state vector, which makes `Y.encodeStateAsUpdate`
 * throw "Unexpected end of array"; the truncated frames run off the end of the
 * buffer inside the decoder.
 */

import assert from 'node:assert/strict'
import test from 'node:test'

import { mintToken, startJwksServer, startMultiplayerServer, waitFor } from './helpers.mjs'
import {
  hasKind,
  messageAwareness,
  messageSync,
  openClient,
  replayInto,
  syncStep1,
  syncStep1Message,
  syncStep2,
  updateMessage
} from './protocol.mjs'

const WORKSPACE = 'test_workspace'
const DOC_PATH = `${WORKSPACE}/f/foo/bar`
// server.mjs closes a connection whose frame it could not decode with 1007
// ("invalid frame payload data").
const INVALID_PAYLOAD = 1007
// Logged by server.mjs for every frame it refused; asserted on so the tests
// cannot pass on a connection that was closed for some unrelated reason.
const REFUSED = 'MALFORMED MESSAGE'

/**
 * 600 KiB of zeros: `messageSync`, then sync step 1 with a zero-length state
 * vector, which `Y.encodeStateAsUpdate` cannot decode.
 */
function zeroFlood() {
  return Buffer.alloc(600 * 1024)
}

/** `messageSync` with nothing after it: `readSyncMessage` reads past the end. */
function truncatedSyncMessage() {
  return Uint8Array.from([messageSync])
}

/** `messageSync` + sync step 1 whose state vector is shorter than it claims. */
function truncatedStateVector() {
  return Uint8Array.from([messageSync, syncStep1, 8, 1, 2, 3])
}

/** `messageAwareness` + an awareness payload shorter than it claims. */
function truncatedAwarenessMessage() {
  return Uint8Array.from([messageAwareness, 8, 1, 2, 3])
}

const MALFORMED = [
  ['600 KiB of zeros', zeroFlood],
  ['a truncated sync message', truncatedSyncMessage],
  ['a truncated sync step 1 state vector', truncatedStateVector],
  ['a truncated awareness update', truncatedAwarenessMessage]
]

/** Connect, wait for the server's sync step 1, i.e. for the peer to be authenticated. */
async function connectAuthenticated(server, token, { onOpen } = {}) {
  const client = openClient(`${server.url}/${DOC_PATH}?token=${token}`, { onOpen })
  await waitFor(() => hasKind(client, syncStep1) || client.closeCode !== undefined, {
    message: 'the server to send sync step 1'
  })
  assert.equal(client.closeCode, undefined, 'connection was closed before it was set up')
  return client
}

for (const [label, payload] of MALFORMED) {
  test(`an authenticated client sending ${label} is closed, and the server survives`, { timeout: 60000 }, async (t) => {
    const jwks = await startJwksServer()
    const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
    t.after(async () => {
      await server.close()
      await jwks.close()
    })

    const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

    // A bystander on the same document, synced before anything goes wrong.
    const bystander = await connectAuthenticated(server, token, {
      onOpen: (ws) => ws.send(syncStep1Message())
    })
    await waitFor(() => hasKind(bystander, syncStep2), {
      message: 'the server to answer the bystander with sync step 2'
    })
    t.after(() => bystander.ws.close())

    // The offender is fully authenticated — the server has already answered it
    // with sync step 1 — so this frame goes through the live message handler,
    // not the pre-auth buffer.
    const offender = await connectAuthenticated(server, token)
    offender.ws.send(payload())

    await waitFor(() => offender.closeCode !== undefined, {
      message: 'the offending connection to be closed'
    })
    assert.equal(offender.closeCode, INVALID_PAYLOAD)
    assert.equal(server.exitStatus, null, `server died: ${server.output}`)
    assert.ok(server.output.includes(REFUSED), `server did not log the refused frame:\n${server.output}`)
    assert.ok(server.output.includes(`doc="${DOC_PATH}"`), 'the refusal must name the document')
    // The payload itself is never logged: 600 KiB of zeros must not reach the log.
    assert.ok(server.output.length < 8192, `server logged ${server.output.length} bytes, payload leaked?`)

    // The bystander was not disturbed, and a new client still syncs.
    assert.equal(bystander.closeCode, undefined)
    const latecomer = await connectAuthenticated(server, token, {
      onOpen: (ws) => ws.send(syncStep1Message())
    })
    t.after(() => latecomer.ws.close())
    await waitFor(() => hasKind(latecomer, syncStep2), {
      message: 'the server to answer a new client with sync step 2'
    })

    // And a real edit still propagates from one client to the other.
    latecomer.ws.send(updateMessage('hello'))
    await waitFor(() => replayInto(bystander).getText('content').toString() === 'hello', {
      message: 'the edit to propagate to the bystander'
    })
  })
}

test('a malformed frame replayed from the pre-auth buffer is refused, not fatal', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ hold: true })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  // The key is parked, so this frame is buffered by the pre-auth path and only
  // decoded later, when `setupWSConnection` replays it. One 600 KiB frame stays
  // under MAX_PREAUTH_BYTES (1 MiB), so the flood cap cannot be what closes it.
  await waitFor(() => jwks.requests >= 1, { message: 'the server to request the JWKS' })
  let framesWritten = 0
  const offender = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws.send(zeroFlood(), () => framesWritten++)
  })
  await waitFor(() => framesWritten === 1, { message: 'the offending client to write its frame' })
  jwks.release()

  await waitFor(() => offender.closeCode !== undefined, {
    message: 'the offending connection to be closed'
  })
  assert.equal(offender.closeCode, INVALID_PAYLOAD)
  assert.equal(server.exitStatus, null, `server died: ${server.output}`)
  assert.ok(server.output.includes(REFUSED), `server did not log the refused frame:\n${server.output}`)

  // The server is unharmed and still syncs a well-behaved client.
  const healthy = await connectAuthenticated(server, token, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })
  t.after(() => healthy.ws.close())
  await waitFor(() => hasKind(healthy, syncStep2), {
    message: 'the server to still answer a well-behaved client with sync step 2'
  })
})
