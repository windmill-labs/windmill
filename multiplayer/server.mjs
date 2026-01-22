#!/usr/bin/env node
/**
 * Simple y-websocket server with connection logging
 * Run with: node server.mjs
 */

import http from 'http'
import { WebSocketServer } from 'ws'
import * as Y from 'yjs'
import * as syncProtocol from 'y-protocols/sync'
import * as awarenessProtocol from 'y-protocols/awareness'
import * as encoding from 'lib0/encoding'
import * as decoding from 'lib0/decoding'

const PORT = process.env.PORT || 3002
const HOST = process.env.HOST || '0.0.0.0'

const messageSync = 0
const messageAwareness = 1

// Store docs in memory
const docs = new Map()

const getYDoc = (docname) => {
  let doc = docs.get(docname)
  if (!doc) {
    doc = new Y.Doc()
    doc.name = docname
    docs.set(docname, doc)
  }
  return doc
}

const send = (conn, message) => {
  if (conn.readyState === 1) { // WebSocket.OPEN
    conn.send(message, err => { if (err) console.error(err) })
  }
}

const setupWSConnection = (conn, req, docName) => {
  const doc = getYDoc(docName)

  // Initialize awareness
  if (!doc.awareness) {
    doc.awareness = new awarenessProtocol.Awareness(doc)
  }

  const awareness = doc.awareness

  // Track connections per doc
  if (!doc.conns) doc.conns = new Set()
  doc.conns.add(conn)

  conn.on('message', (message) => {
    const data = new Uint8Array(message)
    const decoder = decoding.createDecoder(data)
    const messageType = decoding.readVarUint(decoder)

    switch (messageType) {
      case messageSync:
        const encoder = encoding.createEncoder()
        encoding.writeVarUint(encoder, messageSync)
        syncProtocol.readSyncMessage(decoder, encoder, doc, null)
        if (encoding.length(encoder) > 1) {
          send(conn, encoding.toUint8Array(encoder))
        }
        break
      case messageAwareness:
        awarenessProtocol.applyAwarenessUpdate(awareness, decoding.readVarUint8Array(decoder), conn)
        break
    }
  })

  // Send initial sync step 1
  {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageSync)
    syncProtocol.writeSyncStep1(encoder, doc)
    send(conn, encoding.toUint8Array(encoder))
  }

  // Send awareness states
  const awarenessStates = awareness.getStates()
  if (awarenessStates.size > 0) {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageAwareness)
    encoding.writeVarUint8Array(encoder, awarenessProtocol.encodeAwarenessUpdate(awareness, Array.from(awarenessStates.keys())))
    send(conn, encoding.toUint8Array(encoder))
  }

  // Broadcast awareness changes
  const awarenessChangeHandler = ({ added, updated, removed }, origin) => {
    const changedClients = added.concat(updated).concat(removed)
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageAwareness)
    encoding.writeVarUint8Array(encoder, awarenessProtocol.encodeAwarenessUpdate(awareness, changedClients))
    const message = encoding.toUint8Array(encoder)
    doc.conns.forEach(c => send(c, message))
  }
  awareness.on('update', awarenessChangeHandler)

  // Broadcast doc updates
  const updateHandler = (update, origin) => {
    const encoder = encoding.createEncoder()
    encoding.writeVarUint(encoder, messageSync)
    syncProtocol.writeUpdate(encoder, update)
    const message = encoding.toUint8Array(encoder)
    doc.conns.forEach(c => {
      if (c !== origin) send(c, message)
    })
  }
  doc.on('update', updateHandler)

  conn.on('close', () => {
    doc.conns.delete(conn)
    awareness.off('update', awarenessChangeHandler)
    doc.off('update', updateHandler)

    // Clean up awareness for this connection
    awarenessProtocol.removeAwarenessStates(awareness, [doc.clientID], null)
  })
}

const server = http.createServer((req, res) => {
  if (req.url === '/' || req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'text/plain' })
    res.end('okay')
  } else {
    res.writeHead(404)
    res.end('not found')
  }
})

const wss = new WebSocketServer({ server })

wss.on('connection', (ws, req) => {
  const docName = req.url?.slice(1).split('?')[0] || 'unknown'
  const clientIp = req.socket.remoteAddress

  console.log(`[${new Date().toISOString()}] CONNECT: doc="${docName}" from=${clientIp}`)

  ws.on('close', () => {
    console.log(`[${new Date().toISOString()}] DISCONNECT: doc="${docName}" from=${clientIp}`)
  })

  setupWSConnection(ws, req, docName)
})

server.listen(PORT, HOST, () => {
  console.log(`[${new Date().toISOString()}] Multiplayer server running at ${HOST}:${PORT}`)
})
