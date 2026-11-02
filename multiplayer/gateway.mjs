#!/usr/bin/env node
/**
 * Lightweight reverse proxy gateway for windmill-extra services.
 *
 * Listens on a single port (default 3000) and dispatches requests
 * to the correct backend service based on URL prefix:
 *
 *   /ws/*       → LSP        (default: localhost:3001)
 *   /ws_mp/*    → Multiplayer (default: localhost:3002)
 *   /ws_debug/* → Debugger    (default: localhost:3003)
 *   everything else → LSP (fallback)
 *
 * The prefix is stripped before forwarding so each service receives
 * clean paths (e.g. /ws_mp/room-name → /room-name on port 3002).
 *
 * Handles both HTTP and WebSocket upgrades.
 *
 * Requests for the language servers carry a backend-signed `?token=` (purpose
 * "lsp"), checked here since the LSP launcher has no verifier of its own. Set
 * REQUIRE_SIGNED_LSP_REQUESTS=false to turn the check off.
 */

import http from 'http'
import { getPublicKey, verifySignedToken } from './jwt.mjs'

const PORT = parseInt(process.env.PORT || process.env.GATEWAY_PORT || '3000')
const HOST = process.env.GATEWAY_HOST || '0.0.0.0'

const LSP_PORT = parseInt(process.env.LSP_PORT || '3001')
const MULTIPLAYER_PORT = parseInt(process.env.MULTIPLAYER_PORT || '3002')
const DEBUGGER_PORT = parseInt(process.env.DEBUGGER_PORT || '3003')
const REQUIRE_SIGNED_LSP_REQUESTS = process.env.REQUIRE_SIGNED_LSP_REQUESTS !== 'false'

// The only LSP paths served without a token: health probes and the editor's
// connectivity check. Everything else that reaches the LSP port, the fallback
// route included, needs one.
const LSP_OPEN_PATHS = new Set(['/', '/health', '/ws/health', '/ws/ping'])

const ROUTES = [
  { prefix: '/ws_mp/', target: MULTIPLAYER_PORT, strip: '/ws_mp' },
  { prefix: '/ws_mp', target: MULTIPLAYER_PORT, strip: '/ws_mp' },
  { prefix: '/ws_debug/', target: DEBUGGER_PORT, strip: '/ws_debug' },
  { prefix: '/ws_debug', target: DEBUGGER_PORT, strip: '/ws_debug' },
  { prefix: '/ws/', target: LSP_PORT, strip: '' },
  { prefix: '/ws', target: LSP_PORT, strip: '' },
]

function resolve (url) {
  for (const r of ROUTES) {
    if (url === r.prefix || url.startsWith(r.prefix + (r.prefix.endsWith('/') ? '' : '/'))) {
      const stripped = r.strip ? url.slice(r.strip.length) || '/' : url
      return { port: r.target, path: stripped }
    }
  }
  // fallback: forward as-is to LSP
  return { port: LSP_PORT, path: url }
}

/** Returns null when the request may be forwarded, or why it may not. */
async function lspRequestRefusal (port, url) {
  if (port !== LSP_PORT || !REQUIRE_SIGNED_LSP_REQUESTS) return null
  const [pathname, query = ''] = url.split('?', 2)
  if (LSP_OPEN_PATHS.has(pathname)) return null
  const token = new URLSearchParams(query).get('token')
  if (!token) return 'no token'
  const { error } = await verifySignedToken(token, 'lsp')
  return error ?? null
}

// ---- HTTP proxy ----

const server = http.createServer(async (clientReq, clientRes) => {
  const { port, path } = resolve(clientReq.url)

  const refusal = await lspRequestRefusal(port, clientReq.url)
  if (refusal) {
    console.warn(`[gateway] REJECTED ${clientReq.url.split('?')[0]}: ${refusal}`)
    clientRes.writeHead(401, { 'Content-Type': 'text/plain' })
    clientRes.end('Unauthorized')
    return
  }

  const opts = {
    hostname: '127.0.0.1',
    port,
    path,
    method: clientReq.method,
    headers: clientReq.headers,
  }

  const proxy = http.request(opts, (proxyRes) => {
    clientRes.writeHead(proxyRes.statusCode, proxyRes.headers)
    proxyRes.pipe(clientRes, { end: true })
  })

  proxy.on('error', (err) => {
    console.error(`[gateway] HTTP proxy error → :${port}${path}: ${err.message}`)
    if (!clientRes.headersSent) {
      clientRes.writeHead(502, { 'Content-Type': 'text/plain' })
    }
    clientRes.end('Bad Gateway')
  })

  clientReq.pipe(proxy, { end: true })
})

// ---- WebSocket proxy (upgrade) ----

server.on('upgrade', async (clientReq, clientSocket, head) => {
  const { port, path } = resolve(clientReq.url)

  clientSocket.on('error', () => clientSocket.destroy())
  const refusal = await lspRequestRefusal(port, clientReq.url)
  if (refusal) {
    console.warn(`[gateway] REJECTED ${clientReq.url.split('?')[0]}: ${refusal}`)
    clientSocket.end('HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n')
    return
  }

  const opts = {
    hostname: '127.0.0.1',
    port,
    path,
    method: 'GET',
    headers: clientReq.headers,
  }

  const proxy = http.request(opts)

  proxy.on('upgrade', (proxyRes, proxySocket, proxyHead) => {
    // Relay the 101 back to the client
    let response = `HTTP/1.1 101 Switching Protocols\r\n`
    for (let i = 0; i < proxyRes.rawHeaders.length; i += 2) {
      response += `${proxyRes.rawHeaders[i]}: ${proxyRes.rawHeaders[i + 1]}\r\n`
    }
    response += '\r\n'

    clientSocket.write(response)

    if (proxyHead.length) {
      clientSocket.write(proxyHead)
    }

    // Bi-directional pipe
    proxySocket.pipe(clientSocket)
    clientSocket.pipe(proxySocket)

    proxySocket.on('error', () => clientSocket.destroy())
    clientSocket.on('error', () => proxySocket.destroy())
  })

  proxy.on('error', (err) => {
    console.error(`[gateway] WS proxy error → :${port}${path}: ${err.message}`)
    clientSocket.destroy()
  })

  proxy.end()
})

// The port the language servers were served on before the gateway, still targeted by
// proxies set up then; served by the same handlers, so with the same checks.
const LEGACY_LSP_PORT = parseInt(process.env.GATEWAY_LEGACY_LSP_PORT || '')
if (LEGACY_LSP_PORT && LEGACY_LSP_PORT !== PORT) {
  const legacy = http.createServer()
  legacy.on('request', (...args) => server.emit('request', ...args))
  legacy.on('upgrade', (...args) => server.emit('upgrade', ...args))
  legacy.listen(LEGACY_LSP_PORT, HOST, () => console.log(`[gateway] also listening on ${HOST}:${LEGACY_LSP_PORT}`))
}

server.listen(PORT, HOST, () => {
  console.log(`[gateway] listening on ${HOST}:${PORT}`)
  console.log(`[gateway]   /ws/*       → :${LSP_PORT}`)
  console.log(`[gateway]   /ws_mp/*    → :${MULTIPLAYER_PORT}`)
  console.log(`[gateway]   /ws_debug/* → :${DEBUGGER_PORT}`)
  console.log(`[gateway]   (fallback)  → :${LSP_PORT}`)
  console.log(`[gateway] Signed LSP requests ${REQUIRE_SIGNED_LSP_REQUESTS ? 'REQUIRED (set REQUIRE_SIGNED_LSP_REQUESTS=false to disable)' : 'DISABLED'}`)
  if (REQUIRE_SIGNED_LSP_REQUESTS) {
    // Warm the JWKS cache so the first editor does not wait on it.
    getPublicKey()
  }
})
