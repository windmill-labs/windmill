/**
 * The gateway forwards a connection to the language servers only with a
 * backend-signed "lsp" token, except on the health and connectivity paths.
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import http from 'node:http'
import path from 'node:path'
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { freePort, mintToken, startJwksServer, waitFor } from './helpers.mjs'

const GATEWAY_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'gateway.mjs')

/** Stands in for the LSP launcher: accepts every upgrade and records its path. */
async function startFakeLsp() {
  const upgraded = []
  const server = http.createServer((_req, res) => res.end('ok'))
  server.on('upgrade', (req, socket) => {
    upgraded.push(req.url)
    socket.end('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n')
  })
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
  return { port: server.address().port, upgraded, close: () => new Promise((r) => server.close(r)) }
}

/** Resolves to the status the gateway answers a plain request with. */
function httpStatus(port, url) {
  return new Promise((resolve, reject) => {
    http.get({ hostname: '127.0.0.1', port, path: url }, (res) => {
      res.resume()
      resolve(res.statusCode)
    }).on('error', reject)
  })
}

/** Resolves to the status the gateway answers an upgrade request with. */
function upgradeStatus(port, url) {
  return new Promise((resolve, reject) => {
    const req = http.request({
      hostname: '127.0.0.1',
      port,
      path: url,
      headers: { Connection: 'Upgrade', Upgrade: 'websocket' }
    })
    req.on('upgrade', (res, socket) => {
      socket.destroy()
      resolve(res.statusCode)
    })
    req.on('response', (res) => {
      res.resume()
      resolve(res.statusCode)
    })
    req.on('error', reject)
    req.end()
  })
}

test('language server upgrades need a signed lsp token', { timeout: 30000 }, async (t) => {
  const jwks = await startJwksServer()
  const lsp = await startFakeLsp()
  const port = await freePort()
  const legacyPort = await freePort()
  let output = ''
  const gateway = spawn(process.execPath, [GATEWAY_PATH], {
    env: {
      ...process.env,
      PORT: String(port),
      GATEWAY_HOST: '127.0.0.1',
      LSP_PORT: String(lsp.port),
      GATEWAY_LEGACY_LSP_PORT: String(legacyPort),
      WINDMILL_BASE_URL: jwks.baseUrl,
      BASE_INTERNAL_URL: '',
      REQUIRE_SIGNED_LSP_REQUESTS: 'true'
    },
    stdio: ['ignore', 'pipe', 'pipe']
  })
  gateway.stdout.on('data', (c) => { output += c })
  gateway.stderr.on('data', (c) => { output += c })
  t.after(async () => {
    gateway.kill('SIGKILL')
    await lsp.close()
    await jwks.close()
  })
  await waitFor(() => output.includes(`listening on 127.0.0.1:${port}`) && output.includes(`${legacyPort}`), {
    message: 'the gateway to listen'
  })

  const lspToken = mintToken(jwks.privateKey, { workspaceId: 'ws', purpose: 'lsp' })
  const multiplayerToken = mintToken(jwks.privateKey, { workspaceId: 'ws' })
  const expired = mintToken(jwks.privateKey, { workspaceId: 'ws', purpose: 'lsp', ttlSecs: -1 })

  assert.equal(await upgradeStatus(port, '/ws/pyright'), 401)
  assert.equal(await upgradeStatus(port, `/ws/pyright?token=${multiplayerToken}`), 401)
  assert.equal(await upgradeStatus(port, `/ws/pyright?token=${expired}`), 401)
  // The fallback route reaches the launcher too.
  assert.equal(await upgradeStatus(port, '/somewhere'), 401)
  assert.deepEqual(lsp.upgraded, [])

  assert.equal(await upgradeStatus(port, `/ws/pyright?token=${lspToken}`), 101)
  assert.equal(await upgradeStatus(port, '/ws/ping'), 101)
  assert.deepEqual(lsp.upgraded, [`/ws/pyright?token=${lspToken}`, '/ws/ping'])

  assert.equal(await httpStatus(port, '/ws/pyright'), 401)
  assert.equal(await httpStatus(port, '/health'), 200)

  // The pre-gateway LSP port applies the same checks.
  assert.equal(await upgradeStatus(legacyPort, '/ws/pyright'), 401)
  assert.equal(await upgradeStatus(legacyPort, `/ws/pyright?token=${lspToken}`), 101)

  // A token signed after the backend's key changed is accepted without a restart.
  jwks.rotate()
  const rotated = mintToken(jwks.privateKey, { workspaceId: 'ws', purpose: 'lsp' })
  assert.equal(await upgradeStatus(port, `/ws/go?token=${rotated}`), 101)
  // A signature no key verifies is refused, with no second refetch within the interval.
  assert.equal(await upgradeStatus(port, `/ws/pyright?token=${lspToken}`), 401)
  assert.equal(jwks.requests, 2)
})
