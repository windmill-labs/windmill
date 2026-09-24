/**
 * Shared helpers for the multiplayer server tests: a fake Windmill JWKS
 * endpoint, multiplayer token minting and a server.mjs child process.
 */

import { spawn } from 'node:child_process'
import crypto from 'node:crypto'
import http from 'node:http'
import net from 'node:net'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SERVER_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'server.mjs')

export function base64url(buffer) {
  return Buffer.from(buffer).toString('base64url')
}

/**
 * Ask the kernel for an unused port. There is an unavoidable gap between giving
 * the port up and the server.mjs child binding it, so `startMultiplayerServer`
 * retries on EADDRINUSE; servers started in-process bind port 0 directly instead.
 */
async function freePort() {
  return new Promise((resolve, reject) => {
    const probe = net.createServer()
    probe.on('error', reject)
    probe.listen(0, '127.0.0.1', () => {
      const { port } = probe.address()
      probe.close(() => resolve(port))
    })
  })
}

/**
 * Mint a multiplayer JWT in exactly the format the backend produces in
 * `sign_multiplayer` (backend/windmill-api-debug/src/lib.rs): an EdDSA JWT with
 * claims workspace_id, email, iat, exp, purpose.
 */
export function mintToken(privateKey, { workspaceId, email = 'test@windmill.dev', ttlSecs = 300 } = {}) {
  const now = Math.floor(Date.now() / 1000)
  const header = base64url(JSON.stringify({ alg: 'EdDSA', typ: 'JWT' }))
  const claims = base64url(
    JSON.stringify({
      workspace_id: workspaceId,
      email,
      iat: now,
      exp: now + ttlSecs,
      purpose: 'multiplayer'
    })
  )
  const message = `${header}.${claims}`
  const signature = base64url(crypto.sign(null, Buffer.from(message), privateKey))
  return `${message}.${signature}`
}

/**
 * A stand-in for the Windmill backend's /api/debug/jwks, serving the public half
 * of an Ed25519 key pair. `delayMs` keeps the response pending long enough that
 * the server is guaranteed to still be fetching the key when a client connects.
 */
export async function startJwksServer({ delayMs = 0 } = {}) {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519')
  const jwk = publicKey.export({ format: 'jwk' })
  let requests = 0
  const pending = new Map()

  const server = http.createServer((req, res) => {
    if (!req.url?.startsWith('/api/debug/jwks')) {
      res.writeHead(404)
      res.end('not found')
      return
    }
    requests++
    const timer = setTimeout(() => {
      pending.delete(res)
      res.writeHead(200, { 'Content-Type': 'application/json' })
      res.end(
        JSON.stringify({
          keys: [{ kty: jwk.kty, crv: jwk.crv, x: jwk.x, kid: 'test', use: 'sig', alg: 'EdDSA' }]
        })
      )
    }, delayMs)
    pending.set(res, timer)
  })

  await new Promise((resolve, reject) => {
    server.on('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  const { port } = server.address()

  return {
    privateKey,
    baseUrl: `http://127.0.0.1:${port}`,
    get requests() {
      return requests
    },
    async close() {
      // Destroy the still-delayed responses rather than only cancelling their
      // timers: server.close() waits for in-flight requests, so a request left
      // hanging would deadlock teardown.
      for (const [res, timer] of pending) {
        clearTimeout(timer)
        res.destroy()
      }
      pending.clear()
      const closed = new Promise((resolve) => server.close(resolve))
      server.closeAllConnections()
      await closed
    }
  }
}

/**
 * Start server.mjs as a child process and resolve once it is listening, retrying
 * if another process grabbed the port between `freePort()` and the child binding.
 */
export async function startMultiplayerServer(env = {}, attemptsLeft = 5) {
  const port = await freePort()
  const child = spawn(process.execPath, [SERVER_PATH], {
    // Pin the settings the tests assert on, so an ambient
    // REQUIRE_SIGNED_MULTIPLAYER_REQUESTS=false cannot turn the rejection tests
    // into false passes, and BASE_INTERNAL_URL cannot stand in for the fake JWKS.
    env: {
      ...process.env,
      REQUIRE_SIGNED_MULTIPLAYER_REQUESTS: 'true',
      BASE_INTERNAL_URL: '',
      PORT: String(port),
      HOST: '127.0.0.1',
      ...env
    },
    stdio: ['ignore', 'pipe', 'pipe']
  })

  let output = ''
  const listening = new Promise((resolve, reject) => {
    const onData = (chunk) => {
      output += chunk.toString()
      if (output.includes('Multiplayer server running')) resolve()
    }
    child.stdout.on('data', onData)
    child.stderr.on('data', onData)
    child.once('error', reject)
    child.once('exit', (code) => reject(new Error(`server exited early (code ${code}):\n${output}`)))
  })

  try {
    await listening
  } catch (error) {
    if (output.includes('EADDRINUSE') && attemptsLeft > 1) {
      return startMultiplayerServer(env, attemptsLeft - 1)
    }
    throw error
  }

  return {
    port,
    url: `ws://127.0.0.1:${port}`,
    get output() {
      return output
    },
    async close() {
      if (child.exitCode !== null) return
      const exited = new Promise((resolve) => child.once('exit', resolve))
      child.kill('SIGKILL')
      await exited
    }
  }
}

/** Poll `predicate` until it is true, or throw after `timeoutMs`. */
export async function waitFor(predicate, { timeoutMs = 15000, intervalMs = 10, message = 'condition' } = {}) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    if (await predicate()) return
    await new Promise((resolve) => setTimeout(resolve, intervalMs))
  }
  throw new Error(`Timed out after ${timeoutMs}ms waiting for ${message}`)
}
