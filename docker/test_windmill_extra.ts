#!/usr/bin/env bun
/**
 * Integration test for the windmill-extra Docker container.
 *
 * Tests all three services:
 * - LSP (Language Server Protocol) - Port 3001
 * - Multiplayer (y-websocket) - Port 3002
 * - Debugger (DAP WebSocket) - Port 3003
 *
 * Configuration via environment variables:
 * - WINDMILL_EXTRA_HOST: Container hostname (default: localhost)
 * - LSP_PORT: LSP service port (default: 3001)
 * - MULTIPLAYER_PORT: Multiplayer service port (default: 3002)
 * - DEBUGGER_PORT: Debugger service port (default: 3003)
 * - SKIP_LSP: Skip LSP tests (default: false)
 * - SKIP_MULTIPLAYER: Skip Multiplayer tests (default: false)
 * - SKIP_DEBUGGER: Skip Debugger tests (default: false)
 *
 * Usage:
 *   # Start the container first
 *   docker run -p 3001:3001 -p 3002:3002 -p 3003:3003 \
 *     -e ENABLE_LSP=true -e ENABLE_MULTIPLAYER=true -e ENABLE_DEBUGGER=true \
 *     windmill-extra
 *
 *   # Run tests
 *   bun run docker/test_windmill_extra.ts
 *
 *   # Or with custom host
 *   WINDMILL_EXTRA_HOST=windmill_extra bun run docker/test_windmill_extra.ts
 */

// Configuration
const HOST = process.env.WINDMILL_EXTRA_HOST || 'localhost'
const LSP_PORT = parseInt(process.env.LSP_PORT || '3001')
const MULTIPLAYER_PORT = parseInt(process.env.MULTIPLAYER_PORT || '3002')
const DEBUGGER_PORT = parseInt(process.env.DEBUGGER_PORT || '3003')
const SKIP_LSP = process.env.SKIP_LSP === 'true'
const SKIP_MULTIPLAYER = process.env.SKIP_MULTIPLAYER === 'true'
const SKIP_DEBUGGER = process.env.SKIP_DEBUGGER === 'true'

// Test result tracking
interface TestResult {
	name: string
	passed: boolean
	error?: string
	duration?: number
}

const results: TestResult[] = []

function log(message: string) {
	console.log(`[TEST] ${message}`)
}

function logSuccess(test: string) {
	console.log(`  ✓ ${test}`)
}

function logFailure(test: string, error?: string) {
	console.log(`  ✗ ${test}${error ? `: ${error}` : ''}`)
}

async function runTest(name: string, fn: () => Promise<void>): Promise<boolean> {
	const start = Date.now()
	try {
		await fn()
		const duration = Date.now() - start
		results.push({ name, passed: true, duration })
		logSuccess(`${name} (${duration}ms)`)
		return true
	} catch (error) {
		const duration = Date.now() - start
		const errorMsg = error instanceof Error ? error.message : String(error)
		results.push({ name, passed: false, error: errorMsg, duration })
		logFailure(name, errorMsg)
		return false
	}
}

// ============================================================================
// LSP Tests
// ============================================================================

async function testLspHealth(): Promise<void> {
	// LSP returns "ok" on the root WebSocket endpoint
	const response = await fetch(`http://${HOST}:${LSP_PORT}/`, {
		method: 'GET',
		signal: AbortSignal.timeout(5000)
	})

	if (response.status !== 200) {
		throw new Error(`LSP health check failed: HTTP ${response.status}`)
	}

	const text = await response.text()
	if (!text.includes('ok')) {
		throw new Error(`LSP health check response unexpected: ${text}`)
	}
}

async function testLspWebSocket(): Promise<void> {
	// Test WebSocket connection to LSP
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			ws.close()
			reject(new Error('LSP WebSocket connection timeout'))
		}, 5000)

		const ws = new WebSocket(`ws://${HOST}:${LSP_PORT}/ws/pyright`)

		ws.onopen = () => {
			clearTimeout(timeout)
			ws.close()
			resolve()
		}

		ws.onerror = () => {
			clearTimeout(timeout)
			reject(new Error('LSP WebSocket connection failed'))
		}
	})
}

async function runLspTests(): Promise<boolean> {
	log('\n=== LSP Service Tests (Port ' + LSP_PORT + ') ===')

	if (SKIP_LSP) {
		log('  Skipping LSP tests (SKIP_LSP=true)')
		return true
	}

	let passed = true
	passed = (await runTest('LSP health check', testLspHealth)) && passed
	passed = (await runTest('LSP WebSocket connection (pyright)', testLspWebSocket)) && passed

	return passed
}

// ============================================================================
// Multiplayer Tests (y-websocket)
// ============================================================================

async function testMultiplayerWebSocket(): Promise<void> {
	// y-websocket accepts WebSocket connections with room names
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			ws.close()
			reject(new Error('Multiplayer WebSocket connection timeout'))
		}, 5000)

		// y-websocket uses room names in the URL path
		const ws = new WebSocket(`ws://${HOST}:${MULTIPLAYER_PORT}/test-room`)

		ws.onopen = () => {
			clearTimeout(timeout)
			// Send a sync message (y-websocket protocol)
			// Just verifying connection works
			ws.close()
			resolve()
		}

		ws.onerror = () => {
			clearTimeout(timeout)
			reject(new Error('Multiplayer WebSocket connection failed'))
		}
	})
}

async function runMultiplayerTests(): Promise<boolean> {
	log('\n=== Multiplayer Service Tests (Port ' + MULTIPLAYER_PORT + ') ===')

	if (SKIP_MULTIPLAYER) {
		log('  Skipping Multiplayer tests (SKIP_MULTIPLAYER=true)')
		return true
	}

	let passed = true
	passed = (await runTest('Multiplayer WebSocket connection', testMultiplayerWebSocket)) && passed

	return passed
}

// ============================================================================
// Debugger Tests (DAP WebSocket)
// ============================================================================

interface DAPMessage {
	seq: number
	type: 'request' | 'response' | 'event'
	command?: string
	event?: string
	request_seq?: number
	success?: boolean
	message?: string
	body?: Record<string, unknown>
	arguments?: Record<string, unknown>
}

class DAPTestClient {
	private ws: WebSocket | null = null
	private seq = 1
	private pendingRequests = new Map<
		number,
		{ resolve: (value: DAPMessage) => void; reject: (error: Error) => void }
	>()
	private events: DAPMessage[] = []
	private output: string[] = []
	private result: unknown = undefined
	private eventHandlers = new Map<string, ((event: DAPMessage) => void)[]>()

	async connect(endpoint: string): Promise<void> {
		const url = `ws://${HOST}:${DEBUGGER_PORT}${endpoint}`

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				reject(new Error('DAP WebSocket connection timeout'))
			}, 5000)

			this.ws = new WebSocket(url)

			this.ws.onopen = () => {
				clearTimeout(timeout)
				resolve()
			}

			this.ws.onerror = () => {
				clearTimeout(timeout)
				reject(new Error('DAP WebSocket connection failed'))
			}

			this.ws.onmessage = (event) => {
				this.handleMessage(event.data as string)
			}

			this.ws.onclose = () => {
				// Connection closed
			}
		})
	}

	disconnect(): void {
		if (this.ws) {
			this.ws.close()
			this.ws = null
		}
	}

	private handleMessage(data: string): void {
		try {
			const msg = JSON.parse(data) as DAPMessage

			if (msg.type === 'response') {
				const pending = this.pendingRequests.get(msg.request_seq!)
				if (pending) {
					this.pendingRequests.delete(msg.request_seq!)
					if (msg.success) {
						pending.resolve(msg)
					} else {
						pending.reject(new Error(msg.message || 'Request failed'))
					}
				}
			} else if (msg.type === 'event') {
				this.events.push(msg)

				if (msg.event === 'output' && msg.body?.output) {
					const out = String(msg.body.output).trim()
					if (out && !out.startsWith('__WINDMILL_RESULT__')) {
						this.output.push(out)
					}
				}

				if (msg.event === 'terminated' && msg.body?.result !== undefined) {
					this.result = msg.body.result
				}

				const handlers = this.eventHandlers.get(msg.event!) || []
				for (const handler of handlers) {
					handler(msg)
				}
			}
		} catch {
			// Ignore parse errors
		}
	}

	private async sendRequest(command: string, args?: Record<string, unknown>): Promise<DAPMessage> {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			throw new Error('Not connected')
		}

		const seq = this.seq++
		const request: DAPMessage = { seq, type: 'request', command, arguments: args }

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pendingRequests.delete(seq)
				reject(new Error(`Request timeout: ${command}`))
			}, 15000)

			this.pendingRequests.set(seq, {
				resolve: (value) => {
					clearTimeout(timeout)
					resolve(value)
				},
				reject: (error) => {
					clearTimeout(timeout)
					reject(error)
				}
			})

			this.ws!.send(JSON.stringify(request))
		})
	}

	waitForEvent(eventName: string, timeout = 10000): Promise<DAPMessage> {
		return new Promise((resolve, reject) => {
			const timer = setTimeout(() => {
				reject(new Error(`Timeout waiting for event: ${eventName}`))
			}, timeout)

			const handler = (event: DAPMessage) => {
				clearTimeout(timer)
				const handlers = this.eventHandlers.get(eventName) || []
				const idx = handlers.indexOf(handler)
				if (idx >= 0) handlers.splice(idx, 1)
				resolve(event)
			}

			if (!this.eventHandlers.has(eventName)) {
				this.eventHandlers.set(eventName, [])
			}
			this.eventHandlers.get(eventName)!.push(handler)
		})
	}

	async initialize() {
		return this.sendRequest('initialize', {
			clientID: 'test',
			linesStartAt1: true,
			columnsStartAt1: true
		})
	}

	async setBreakpoints(path: string, lines: number[]) {
		return this.sendRequest('setBreakpoints', {
			source: { path },
			breakpoints: lines.map((line) => ({ line }))
		})
	}

	async configurationDone() {
		return this.sendRequest('configurationDone')
	}

	async launch(code: string, callMain = false, args: Record<string, unknown> = {}) {
		return this.sendRequest('launch', { code, callMain, args })
	}

	async continue_() {
		return this.sendRequest('continue', { threadId: 1 })
	}

	async terminate() {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			return null
		}
		return this.sendRequest('terminate')
	}

	getOutput() {
		return this.output
	}

	getResult() {
		return this.result
	}

	clearState() {
		this.output = []
		this.result = undefined
		this.events = []
	}
}

async function testDebuggerHealth(): Promise<void> {
	const response = await fetch(`http://${HOST}:${DEBUGGER_PORT}/health`, {
		method: 'GET',
		signal: AbortSignal.timeout(5000)
	})

	if (response.status !== 200) {
		throw new Error(`Debugger health check failed: HTTP ${response.status}`)
	}

	const data = await response.json()
	if (data.status !== 'ok') {
		throw new Error(`Debugger health status: ${data.status}`)
	}

	// Verify endpoints are listed
	if (!data.endpoints || !Array.isArray(data.endpoints)) {
		throw new Error('Debugger health missing endpoints')
	}

	const requiredEndpoints = ['/python', '/typescript', '/bun']
	for (const ep of requiredEndpoints) {
		if (!data.endpoints.includes(ep)) {
			throw new Error(`Debugger missing endpoint: ${ep}`)
		}
	}
}

async function testDebuggerTypescriptExecution(): Promise<void> {
	const client = new DAPTestClient()

	try {
		await client.connect('/typescript')
		await client.initialize()

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()

		const code = `export async function main(name: string) {
  console.log("Hello " + name)
  return { greeting: "Hello " + name }
}`

		await client.launch(code, true, { name: 'World' })
		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {
			// May have already terminated
		}

		await new Promise((r) => setTimeout(r, 500))

		const output = client.getOutput()
		const result = client.getResult() as { greeting?: string } | undefined

		if (!output.some((o) => o.includes('Hello World'))) {
			throw new Error(`Missing expected output. Got: ${JSON.stringify(output)}`)
		}

		if (!result || result.greeting !== 'Hello World') {
			throw new Error(`Incorrect result. Got: ${JSON.stringify(result)}`)
		}
	} finally {
		client.disconnect()
	}
}

async function testDebuggerPythonExecution(): Promise<void> {
	const client = new DAPTestClient()

	try {
		await client.connect('/python')
		await client.initialize()

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.py', [])
		await client.configurationDone()

		const code = `def main(name):
    print(f"Hello {name}")
    return {"greeting": f"Hello {name}"}
`

		await client.launch(code, true, { name: 'World' })
		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {
			// May have already terminated
		}

		await new Promise((r) => setTimeout(r, 500))

		const output = client.getOutput()
		const result = client.getResult() as { greeting?: string } | undefined

		if (!output.some((o) => o.includes('Hello World'))) {
			throw new Error(`Missing expected output. Got: ${JSON.stringify(output)}`)
		}

		if (!result || result.greeting !== 'Hello World') {
			throw new Error(`Incorrect result. Got: ${JSON.stringify(result)}`)
		}
	} finally {
		client.disconnect()
	}
}

async function testDebuggerBreakpoints(): Promise<void> {
	const client = new DAPTestClient()

	try {
		await client.connect('/typescript')
		await client.initialize()

		const initP = client.waitForEvent('initialized')

		// Set breakpoint on line 3 (console.log)
		await client.setBreakpoints('/test.ts', [3])
		await client.configurationDone()

		const code = `export async function main() {
  let x = 1
  console.log("At breakpoint")
  return x
}`

		await client.launch(code, true, {})
		await initP

		// Wait for stopped at breakpoint
		const stopped = await client.waitForEvent('stopped', 15000)

		if (stopped.body?.reason !== 'breakpoint') {
			throw new Error(`Expected stop reason 'breakpoint', got '${stopped.body?.reason}'`)
		}

		// Continue execution
		await client.continue_()

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 5000)
		} catch {
			// May have already terminated
		}

		await new Promise((r) => setTimeout(r, 500))

		const result = client.getResult()
		if (result !== 1) {
			throw new Error(`Expected result 1, got ${JSON.stringify(result)}`)
		}
	} finally {
		client.disconnect()
	}
}

async function runDebuggerTests(): Promise<boolean> {
	log('\n=== Debugger Service Tests (Port ' + DEBUGGER_PORT + ') ===')

	if (SKIP_DEBUGGER) {
		log('  Skipping Debugger tests (SKIP_DEBUGGER=true)')
		return true
	}

	let passed = true
	passed = (await runTest('Debugger health check', testDebuggerHealth)) && passed
	passed = (await runTest('Debugger TypeScript execution', testDebuggerTypescriptExecution)) && passed
	passed = (await runTest('Debugger Python execution', testDebuggerPythonExecution)) && passed
	passed = (await runTest('Debugger breakpoint support', testDebuggerBreakpoints)) && passed

	return passed
}

// ============================================================================
// Main Test Runner
// ============================================================================

async function waitForServices(maxWait = 30000): Promise<void> {
	log('Waiting for services to be ready...')
	const start = Date.now()

	const checkService = async (name: string, url: string): Promise<boolean> => {
		try {
			const response = await fetch(url, {
				method: 'GET',
				signal: AbortSignal.timeout(2000)
			})
			return response.status === 200
		} catch {
			return false
		}
	}

	while (Date.now() - start < maxWait) {
		const checks = await Promise.all([
			SKIP_LSP || checkService('LSP', `http://${HOST}:${LSP_PORT}/`),
			SKIP_MULTIPLAYER || true, // y-websocket doesn't have a health endpoint
			SKIP_DEBUGGER || checkService('Debugger', `http://${HOST}:${DEBUGGER_PORT}/health`)
		])

		if (checks.every((c) => c)) {
			log('All services are ready!')
			return
		}

		await new Promise((r) => setTimeout(r, 1000))
	}

	throw new Error('Timeout waiting for services to be ready')
}

async function main() {
	console.log('='.repeat(60))
	console.log('Windmill Extra Integration Tests')
	console.log('='.repeat(60))
	console.log(`Host: ${HOST}`)
	console.log(`LSP Port: ${LSP_PORT} (${SKIP_LSP ? 'SKIPPED' : 'enabled'})`)
	console.log(`Multiplayer Port: ${MULTIPLAYER_PORT} (${SKIP_MULTIPLAYER ? 'SKIPPED' : 'enabled'})`)
	console.log(`Debugger Port: ${DEBUGGER_PORT} (${SKIP_DEBUGGER ? 'SKIPPED' : 'enabled'})`)
	console.log('='.repeat(60))

	try {
		await waitForServices()
	} catch (error) {
		console.error(`\n✗ ${error instanceof Error ? error.message : error}`)
		console.error('\nMake sure the windmill-extra container is running:')
		console.error('  docker run -p 3001:3001 -p 3002:3002 -p 3003:3003 \\')
		console.error('    -e ENABLE_LSP=true -e ENABLE_MULTIPLAYER=true -e ENABLE_DEBUGGER=true \\')
		console.error('    windmill-extra')
		process.exit(1)
	}

	let allPassed = true

	// Run LSP tests
	allPassed = (await runLspTests()) && allPassed

	// Run Multiplayer tests
	allPassed = (await runMultiplayerTests()) && allPassed

	// Run Debugger tests
	allPassed = (await runDebuggerTests()) && allPassed

	// Summary
	console.log('\n' + '='.repeat(60))
	console.log('TEST SUMMARY')
	console.log('='.repeat(60))

	const passed = results.filter((r) => r.passed).length
	const failed = results.filter((r) => !r.passed).length

	console.log(`\nTotal: ${results.length} tests`)
	console.log(`Passed: ${passed}`)
	console.log(`Failed: ${failed}`)

	if (failed > 0) {
		console.log('\nFailed tests:')
		for (const r of results.filter((r) => !r.passed)) {
			console.log(`  - ${r.name}: ${r.error || 'Unknown error'}`)
		}
	}

	console.log('\n' + '='.repeat(60))

	if (failed > 0) {
		console.log('✗ Some tests failed')
		process.exit(1)
	} else {
		console.log('✓ All tests passed!')
		process.exit(0)
	}
}

main().catch((error) => {
	console.error('Test runner failed:', error)
	process.exit(1)
})
