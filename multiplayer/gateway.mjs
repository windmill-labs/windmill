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
 */

import http from 'http'

const PORT = parseInt(process.env.GATEWAY_PORT || '3000')
const HOST = process.env.GATEWAY_HOST || '0.0.0.0'

const LSP_PORT = parseInt(process.env.LSP_PORT || '3001')
const MULTIPLAYER_PORT = parseInt(process.env.MULTIPLAYER_PORT || '3002')
const DEBUGGER_PORT = parseInt(process.env.DEBUGGER_PORT || '3003')

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

// ---- HTTP proxy ----

const server = http.createServer((clientReq, clientRes) => {
  const { port, path } = resolve(clientReq.url)

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

server.on('upgrade', (clientReq, clientSocket, head) => {
  const { port, path } = resolve(clientReq.url)

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

server.listen(PORT, HOST, () => {
  console.log(`[gateway] listening on ${HOST}:${PORT}`)
  console.log(`[gateway]   /ws/*       → :${LSP_PORT}`)
  console.log(`[gateway]   /ws_mp/*    → :${MULTIPLAYER_PORT}`)
  console.log(`[gateway]   /ws_debug/* → :${DEBUGGER_PORT}`)
  console.log(`[gateway]   (fallback)  → :${LSP_PORT}`)
})
