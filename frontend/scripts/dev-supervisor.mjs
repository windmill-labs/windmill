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
//
// Proxying is at the TCP layer so HTTP, the HMR websocket, and the /api proxy all
// pass through untouched.
import net from 'node:net'
import { spawn } from 'node:child_process'
import { appendFileSync, existsSync, readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'

const START_TIMEOUT_MS = 180_000
const POLL_INTERVAL_MS = 250
const TICK_MS = 15_000

function parseDuration(text) {
	const m = /^(\d+)(ms|s|m|h)?$/.exec(text)
	if (!m) throw new Error(`invalid duration: ${text}`)
	const scale = { ms: 1, s: 1000, m: 60_000, h: 3_600_000 }[m[2] ?? 'm']
	return Number(m[1]) * scale
}

function parseArgs(argv) {
	const opts = { targets: [], idleMs: parseDuration('15m'), bind: '0.0.0.0', stats: null }
	for (let i = 0; i < argv.length; i++) {
		const arg = argv[i]
		const next = () => {
			const value = argv[++i]
			if (value === undefined) throw new Error(`${arg} needs a value`)
			return value
		}
		if (arg === '-t' || arg === '--target') {
			const [port, cwd] = next().split(':')
			opts.targets.push({ port: Number(port), cwd: path.resolve(cwd ?? process.cwd()) })
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
	let children
	try {
		children = new Map()
		for (const entry of readdirSync('/proc')) {
			if (!/^\d+$/.test(entry)) continue
			const stat = readFileSync(`/proc/${entry}/stat`, 'utf8')
			const ppid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1])
			if (!children.has(ppid)) children.set(ppid, [])
			children.get(ppid).push(Number(entry))
		}
	} catch {
		return null
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
		this.lastActivity = Date.now()
		this.liveSockets = new Set()
		this.stopping = false
	}

	log(message) {
		console.log(`[${this.port} ${this.name}] ${message}`)
	}

	async ensureStarted() {
		if (this.child && this.internalPort) return this.internalPort
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
				stdio: ['ignore', 'pipe', 'pipe']
			}
		)
		this.child = child
		this.internalPort = internalPort
		this.stopping = false
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
			if (!this.stopping) this.log(`dev server exited unexpectedly (${signal ?? code})`)
			if (this.child === child) {
				this.child = null
				this.internalPort = null
			}
		})

		this.log(`starting dev server on :${internalPort}`)
		const started = Date.now()
		while (Date.now() - started < START_TIMEOUT_MS) {
			if (!this.child) throw new Error('dev server exited during startup')
			if (await canConnect(internalPort)) {
				this.log(`ready in ${((Date.now() - started) / 1000).toFixed(1)}s`)
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
		this.child.kill('SIGTERM')
		const child = this.child
		setTimeout(() => child.kill('SIGKILL'), 5000).unref()
		this.child = null
		this.internalPort = null
		for (const socket of this.liveSockets) socket.destroy()
		this.liveSockets.clear()
	}

	handle(client) {
		client.on('error', () => client.destroy())
		client.once('data', (first) => {
			client.pause()
			// A lone HMR socket from a tab left open is not someone looking at the page, so
			// websocket bytes neither hold the dev server alive nor start it. Vite's client
			// probes for a restart with a websocket too (subprotocol `vite-ping`), so starting
			// on one would have the reconnect loop resurrect every server we just stopped.
			const head = first.toString('latin1', 0, Math.min(first.length, 2048))
			const isWebsocket = /\r\nupgrade:\s*websocket/i.test(head)
			if (isWebsocket && !this.child) {
				client.destroy()
				return
			}
			if (!isWebsocket) {
				this.lastActivity = Date.now()
				this.liveSockets.add(client)
			}

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
		})
	}

	tick() {
		if (!this.child) return
		if (this.liveSockets.size > 0) return
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
	if (opts.stats) {
		const now = new Date().toISOString()
		for (const target of targets) {
			appendFileSync(opts.stats, JSON.stringify({ at: now, ...target.sample() }) + '\n')
		}
	}
}, TICK_MS).unref()

for (const signal of ['SIGINT', 'SIGTERM']) {
	process.on(signal, () => {
		for (const target of targets) target.stop()
		process.exit(0)
	})
}

console.log(
	`supervising ${targets.length} target(s), idle timeout ${Math.round(opts.idleMs / 60_000)}m`
)
