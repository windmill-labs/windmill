/**
 * Regression tests for WebSocket-level protocol errors.
 *
 * A frame the `ws` library itself cannot parse — an unmasked frame from a
 * client, a reserved opcode, a bad RSV bit — never reaches the application's
 * 'message' handler. `ws` fails it in its Receiver and emits an 'error' on the
 * WebSocket (`receiverOnError` in ws/lib/websocket.js), and an unhandled 'error'
 * on an EventEmitter throws, so the process used to exit here too — the same
 * blast radius as a malformed application payload, reached one layer lower.
 *
 * Raw bytes are written straight to the TCP socket, since the `ws` client would
 * never produce an illegal frame on its own.
 */

import assert from 'node:assert/strict'
import test from 'node:test'

import { mintToken, startJwksServer, startMultiplayerServer, waitFor } from './helpers.mjs'
import { hasKind, openClient, syncStep1, syncStep1Message, syncStep2 } from './protocol.mjs'

const WORKSPACE = 'test_workspace'
const DOC_PATH = `${WORKSPACE}/f/foo/bar`
// Logged by server.mjs for every connection `ws` failed at the protocol level.
const SOCKET_ERROR = 'SOCKET ERROR'

/**
 * A FIN + text frame of 3 bytes with the MASK bit clear. RFC 6455 requires every
 * client-to-server frame to be masked, so `ws` rejects it with
 * WS_ERR_EXPECTED_MASK (close status 1002).
 */
const UNMASKED_FRAME = Buffer.from([0x81, 0x03, 0x61, 0x62, 0x63])

/** Assert the server is alive by making it serve a fresh client. */
async function assertStillServing(t, server, token) {
  const healthy = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws.send(syncStep1Message())
  })
  t.after(() => healthy.ws.close())
  // Resolve as soon as either outcome is settled, so a dead server fails fast
  // and with its own output rather than by timing out.
  await waitFor(() => hasKind(healthy, syncStep2) || server.exitStatus !== null, {
    message: 'the server to answer a new client with sync step 2'
  })
  assert.equal(server.exitStatus, null, `server died: ${server.output}`)
  assert.ok(hasKind(healthy, syncStep2))
}

test('an illegal WebSocket frame from an authenticated client does not exit the server', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer()
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  const offender = openClient(`${server.url}/${DOC_PATH}?token=${token}`)
  // The server's sync step 1 means this peer is past authentication.
  await waitFor(() => hasKind(offender, syncStep1), { message: 'the server to send sync step 1' })
  offender.ws._socket.write(UNMASKED_FRAME)

  await waitFor(() => offender.closeCode !== undefined, {
    message: 'the offending connection to be closed'
  })
  assert.equal(server.exitStatus, null, `server died: ${server.output}`)
  assert.ok(server.output.includes(SOCKET_ERROR), `server did not log the socket error:\n${server.output}`)

  await assertStillServing(t, server, token)
})

test('an illegal WebSocket frame before authentication does not exit the server', { timeout: 60000 }, async (t) => {
  const jwks = await startJwksServer({ hold: true })
  const server = await startMultiplayerServer({ WINDMILL_BASE_URL: jwks.baseUrl })
  t.after(async () => {
    await server.close()
    await jwks.close()
  })

  const token = mintToken(jwks.privateKey, { workspaceId: WORKSPACE })

  // The JWKS response is parked, so this connection is still unauthenticated and
  // `setupWSConnection` has not run for it.
  await waitFor(() => jwks.requests >= 1, { message: 'the server to request the JWKS' })
  const offender = openClient(`${server.url}/${DOC_PATH}?token=${token}`, {
    onOpen: (ws) => ws._socket.write(UNMASKED_FRAME)
  })

  await waitFor(() => offender.closeCode !== undefined, {
    message: 'the offending connection to be closed'
  })
  assert.equal(server.exitStatus, null, `server died: ${server.output}`)
  assert.ok(server.output.includes(SOCKET_ERROR), `server did not log the socket error:\n${server.output}`)

  jwks.release()
  await assertStillServing(t, server, token)
})
