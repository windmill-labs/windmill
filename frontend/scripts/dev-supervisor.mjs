#!/usr/bin/env node
// Keeps a worktree's `vite dev` off the machine until someone actually looks at it.
//
// The supervisor owns the public port, spawns the dev server on first connection,
// proxies to it, and kills it once traffic stops. A dev server for this frontend
// costs 1.1-1.7 GB resident once browsed, so with several worktrees in flight the
// ones nobody has open are the bulk of the cost.
//
//   node scripts/dev-supervisor.mjs                        # this worktree, $FRONTEND_PORT
//   node scripts/dev-supervisor.mjs -t 3340:/path/to/wt -t 3350:/other
//   node scripts/dev-supervisor.mjs -t 3340:/path --idle 10m --stats rss.jsonl
//   node scripts/dev-supervisor.mjs -t 3340:/path --bind 0.0.0.0   # reachable off-host
//
// Proxying is at the TCP layer so HTTP, the HMR websocket, and the /api proxy all
// pass through untouched. Under HTTPS=true the request head is encrypted, so an HMR
// socket cannot be told apart from real traffic: a target with no tab open is still
// reclaimed, but a tab left open keeps its server alive rather than going dormant.
import net from 'node:net'
import { spawn } from 'node:child_process'
import { appendFileSync, existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

const START_TIMEOUT_MS = 180_000
const POLL_INTERVAL_MS = 250
const TICK_MS = 15_000
const KILL_GRACE_MS = 5_000
const MAX_HEAD_BYTES = 8_192
const HEAD_TIMEOUT_MS = 30_000
const TLS_HANDSHAKE_BYTE = 0x16

// Every spawned dev server, so no exit path can orphan one.
const liveChildren = new Set()

// Children are spawned detached, so signalling the negated pid reaches vite's own workers
// too. Without that, a wedged vite leaves the esbuild/rollup processes this tool exists to
// reclaim. Falls back to the direct child if the group is already gone.
function killTree(child, signal) {
	try {
		process.kill(-child.pid, signal)
	} catch {
		try {
			child.kill(signal)
		} catch {
			// already exited
		}
	}
}

function parseDuration(text) {
	const m = /^(\d+)(ms|s|m|h)?$/.exec(text)
	if (!m) throw new Error(`invalid duration: ${text}`)
	const scale = { ms: 1, s: 1000, m: 60_000, h: 3_600_000 }[m[2] ?? 'm']
	return Number(m[1]) * scale
}

function parseArgs(argv) {
	// Loopback by default: the supervised port fronts the /api proxy to a local backend
	// and serves source over /@fs, and `server.allowedHosts` does not stop a non-browser
	// client from sending whatever Host header it likes. Widening is opt-in.
	// Keep `--idle` above the app's 5-minute background poll. Below it, a tab that never
	// went dormant can have its server reclaimed with no way back: websockets cannot start
	// one, and the dormancy warm-up only fires on a dormant-to-awake transition.
	const opts = { targets: [], idleMs: parseDuration('15m'), bind: '127.0.0.1', stats: null }
	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i]
		const next = () => {
			const value = argv[++i]
			if (value === undefined) throw new Error(`${arg} needs a value`)
			return value
		}
		if (arg === '-t' || arg === '--target') {
			// First colon only: a path may contain one, and an unvalidated port would reach
			// `listen(NaN)`, which quietly binds a random port instead of failing.
			const value = next()
			const split = value.indexOf(':')
			const port = Number(split === -1 ? value : value.slice(0, split))
			if (!Number.isInteger(port) || port < 1 || port > 65535) {
				throw new Error(`--target needs <port>[:<cwd>], got: ${value}`)
			}
			const cwd = split === -1 ? process.cwd() : value.slice(split + 1)
			opts.targets.push({ port, cwd: path.resolve(cwd) })
		} else if (arg === '--idle') opts.idleMs = parseDuration(next())
		else if (arg === '--bind') opts.bind = next()
		else if (arg === '--stats') opts.stats = path.resolve(next())
		else throw new Error(`unknown argument: ${arg}`)
	}
	if (opts.targets.length === 0) {
		opts.targets.push({
			port: Number(process.env.FRONTEND_PORT ?? 3000),
			cwd: path.resolve(process.cwd())
		})
	}
	return opts
}

function freePort() {
	return new Promise((resolve, reject) => {
		const probe = net.createServer()
		probe.on('error', reject)
		probe.listen(0, '127.0.0.1', () => {
			const { port } = probe.address()
			probe.close(() => resolve(port))
		})
	})
}

function canConnect(port) {
	return new Promise((resolve) => {
		const socket = net.connect(port, '127.0.0.1')
		const settle = (ok) => {
			socket.destroy()
			resolve(ok)
		}
		socket.on('connect', () => settle(true))
		socket.on('error', () => settle(false))
	})
}

// RSS of the child's whole process tree: vite spawns workers, and the number worth
// reporting is what the machine gives back when the tree is killed.
function treeRssMb(rootPid) {
	const children = new Map()
	let entries
	try {
		entries = readdirSync('/proc')
	} catch {
		return null
	}
	for (const entry of entries) {
		if (!/^\d+$/.test(entry)) continue
		// Processes come and go mid-scan, so one unreadable pid must not lose the whole
		// tree: a null here would silently report a running server as 0 MB.
		let stat
		try {
			stat = readFileSync(`/proc/${entry}/stat`, 'utf8')
		} catch {
			continue
		}
		const ppid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1])
		if (!children.has(ppid)) children.set(ppid, [])
		children.get(ppid).push(Number(entry))
	}
	let total = 0
	const stack = [rootPid]
	while (stack.length) {
		const pid = stack.pop()
		try {
			const status = readFileSync(`/proc/${pid}/status`, 'utf8')
			total += Number(/VmRSS:\s+(\d+)/.exec(status)?.[1] ?? 0)
		} catch {
			continue
		}
		stack.push(...(children.get(pid) ?? []))
	}
	return Math.round(total / 1024)
}

class Target {
	constructor({ port, cwd }, opts) {
		this.port = port
		this.cwd = cwd
		this.opts = opts
		this.name = path.basename(path.dirname(cwd)) + '/' + path.basename(cwd)
		this.child = null
		this.internalPort = null
		this.starting = null
		this.ready = false
		this.lastActivity = Date.now()
		this.liveSockets = new Set()
		this.stopping = false
	}

	log(message) {
		console.log(`[${this.port} ${this.name}] ${message}`)
	}

	async ensureStarted() {
		// `ready`, not `child`: the port is only connectable once vite has bound it, and a
		// second request arriving mid-startup would otherwise be handed a dead port.
		if (this.child && this.ready) return this.internalPort
		if (this.starting) return this.starting
		this.starting = this.#start().finally(() => {
			this.starting = null
		})
		return this.starting
	}

	async #start() {
		const bin = path.join(this.cwd, 'node_modules/.bin/vite')
		if (!existsSync(bin)) throw new Error(`no vite binary at ${bin}`)
		const internalPort = await freePort()
		// --strictPort so a port race fails loudly instead of vite silently binding
		// elsewhere and every proxied request hanging. --host pins the child to v4
		// loopback: vite's default `localhost` can resolve to ::1 only, which the
		// readiness probe and the proxy would never reach.
		const child = spawn(
			bin,
			['dev', '--port', String(internalPort), '--strictPort', '--host', '127.0.0.1'],
			{
				cwd: this.cwd,
				env: { ...process.env, FRONTEND_PORT: String(internalPort) },
				stdio: ['ignore', 'pipe', 'pipe'],
				detached: true
			}
		)
		this.child = child
		this.internalPort = internalPort
		this.ready = false
		this.stopping = false
		liveChildren.add(child)
		const relay = (stream) => {
			let buffered = ''
			stream.setEncoding('utf8')
			stream.on('data', (chunk) => {
				buffered += chunk
				const lines = buffered.split('\n')
				buffered = lines.pop() ?? ''
				for (const line of lines) if (line.trim()) this.log(`  ${line}`)
			})
		}
		relay(child.stdout)
		relay(child.stderr)
		child.on('exit', (code, signal) => {
			liveChildren.delete(child)
			if (!this.stopping) this.log(`dev server exited unexpectedly (${signal ?? code})`)
			if (this.child === child) {
				this.child = null
				this.internalPort = null
				this.ready = false
			}
		})

		this.log(`starting dev server on :${internalPort}`)
		const started = Date.now()
		while (Date.now() - started < START_TIMEOUT_MS) {
			if (!this.child) throw new Error('dev server exited during startup')
			if (await canConnect(internalPort)) {
				this.log(`ready in ${((Date.now() - started) / 1000).toFixed(1)}s`)
				this.ready = true
				this.lastActivity = Date.now()
				return internalPort
			}
			await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS))
		}
		this.stop()
		throw new Error('dev server did not become reachable')
	}

	stop() {
		if (!this.child) return
		this.stopping = true
		const rss = treeRssMb(this.child.pid)
		this.log(`stopping dev server${rss ? ` (reclaiming ${rss} MB)` : ''}`)
		const child = this.child
		killTree(child, 'SIGTERM')
		// Only if it is still ours to kill: the pid may have been recycled by then, and the
		// group signal would land on an unrelated process group.
		setTimeout(() => {
			if (liveChildren.has(child)) killTree(child, 'SIGKILL')
		}, KILL_GRACE_MS).unref()
		this.child = null
		this.internalPort = null
		this.ready = false
		for (const socket of this.liveSockets) socket.destroy()
		this.liveSockets.clear()
	}

	handle(client) {
		client.on('error', () => client.destroy())
		// TCP preserves no message boundaries, so the request head can arrive split across
		// segments: classify only once the header terminator is in hand, otherwise an upgrade
		// split mid-header reads as ordinary traffic and the HMR socket keeps a server alive
		// that nobody is watching. A TLS record (HTTPS=true) never contains that terminator,
		// so it is dispatched opaquely rather than waited on forever.
		const chunks = []
		let buffered = 0
		let dispatched = false
		const dispatch = (head) => {
			dispatched = true
			clearTimeout(headTimer)
			client.off('data', onData)
			client.pause()
			this.#dispatch(client, head)
		}
		const onData = (chunk) => {
			chunks.push(chunk)
			buffered += chunk.length
			const head = chunks.length === 1 ? chunks[0] : Buffer.concat(chunks)
			if (head.length > 0 && head[0] === TLS_HANDSHAKE_BYTE) return dispatch(head)
			if (head.indexOf('\r\n\r\n') === -1 && buffered < MAX_HEAD_BYTES) return
			dispatch(head)
		}
		// Nothing may sit here forever: a speculative preconnect that never sends a request
		// would otherwise leak a socket per attempt.
		const headTimer = setTimeout(() => {
			if (!dispatched) client.destroy()
		}, HEAD_TIMEOUT_MS)
		headTimer.unref()
		client.on('data', onData)
	}

	#dispatch(client, first) {
		// No websocket may start a stopped server, nor count as activity: every websocket
		// client here reconnects on an unconditional timer (y-websocket at a 2.5s ceiling),
		// so starting on one keeps a reclaimed server alive for as long as a tab is open.
		// `devPollingDormancy` warms it over HTTP on return instead, before they retry.
		const head = first.toString('latin1', 0, Math.min(first.length, MAX_HEAD_BYTES))
		const isWebsocket = /\r\nupgrade:\s*websocket/i.test(head)
		// `starting` too, not just `child`: a start is in flight before the child exists, and
		// the warm-up opens exactly that window for the sockets retrying alongside it. A
		// socket may join a start someone else asked for; it still never initiates one.
		if (isWebsocket && !this.child && !this.starting) {
			client.destroy()
			return
		}
		if (!isWebsocket) {
			this.lastActivity = Date.now()
		}
		this.liveSockets.add(client)
		// Registered at insertion, not after the upstream connects: a client that gives up
		// during a cold start would otherwise stay in the set forever and silently wedge the
		// idle reaper for the life of the process.
		client.once('close', () => this.liveSockets.delete(client))

		this.ensureStarted().then(
			(port) => {
				const upstream = net.connect(port, '127.0.0.1', () => {
					upstream.write(first)
					client.pipe(upstream)
					upstream.pipe(client)
					client.resume()
				})
				const bump = isWebsocket ? () => {} : () => (this.lastActivity = Date.now())

				client.on('data', bump)
				upstream.on('data', bump)
				const teardown = () => {
					this.liveSockets.delete(client)
					client.destroy()
					upstream.destroy()
				}
				upstream.on('error', teardown)
				upstream.on('close', teardown)
				client.on('close', teardown)
			},
			(err) => {
				this.log(`failed to start: ${err.message}`)
				this.liveSockets.delete(client)
				client.destroy()
			}
		)
	}

	tick() {
		if (!this.child) return
		// No bytes flow while vite boots, so an `--idle` shorter than a cold start would
		// otherwise reap the server the waiting client is still queued behind.
		if (this.starting) return
		// Staleness alone, deliberately: an open socket is not proof of use, and gating on
		// one being present lets a half-open connection (VPN drop, suspend), an idle SSE
		// stream, or an opaque TLS socket pin the server forever. Every byte in either
		// direction bumps lastActivity, so anything genuinely in flight keeps this fresh.
		if (Date.now() - this.lastActivity < this.opts.idleMs) return
		this.log(`idle for ${Math.round(this.opts.idleMs / 60_000)}m`)
		this.stop()
	}

	sample() {
		if (!this.child) return { port: this.port, cwd: this.cwd, running: false, rssMb: 0 }
		return {
			port: this.port,
			cwd: this.cwd,
			running: true,
			rssMb: treeRssMb(this.child.pid) ?? 0
		}
	}
}

const opts = parseArgs(process.argv.slice(2))
const targets = opts.targets.map((t) => new Target(t, opts))

for (const target of targets) {
	const server = net.createServer((client) => target.handle(client))
	server.on('error', (err) => {
		console.error(`[${target.port}] listen failed: ${err.message}`)
		process.exit(1)
	})
	server.listen(target.port, opts.bind, () => target.log(`supervising ${target.cwd}`))
}

setInterval(() => {
	for (const target of targets) target.tick()
	if (!opts.stats) return
	const now = new Date().toISOString()
	try {
		for (const target of targets) {
			appendFileSync(opts.stats, JSON.stringify({ at: now, ...target.sample() }) + '\n')
		}
	} catch (err) {
		// An unwritable stats path must not throw out of the tick: that would leave the
		// dev servers running with nothing left to reap them.
		console.error(`stats write failed, continuing: ${err.message}`)
	}
}, TICK_MS).unref()

// Whatever route we leave by, the dev servers are the thing this tool exists to
// reclaim, so nothing may outlive the supervisor.
process.on('exit', () => {
	for (const child of liveChildren) killTree(child, 'SIGKILL')
})

// SIGHUP included: as a tmux pane the supervisor is hung up when the pane closes, and
// without a listener node dies on the default disposition without running `exit` — which
// would strand the detached child, since it has its own session and misses the hangup.
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
	process.on(signal, async () => {
		for (const target of targets) target.stop()
		const deadline = Date.now() + KILL_GRACE_MS
		while (liveChildren.size > 0 && Date.now() < deadline) {
			await new Promise((r) => setTimeout(r, 50))
		}
		process.exit(0)
	})
}

console.log(
	`supervising ${targets.length} target(s), idle timeout ${Math.round(opts.idleMs / 60_000)}m`
)
