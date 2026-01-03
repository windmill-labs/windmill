#!/usr/bin/env bun
/**
 * Test script for the unified DAP Debug Service.
 *
 * Tests both TypeScript/Bun and Python debugging endpoints.
 *
 * Run the service first:
 *    bun run dap_debug_service.ts
 *
 * Then run this test:
 *    bun run test_debug_service.ts
 */

const SERVICE_URL = 'ws://localhost:5679'

const TS_TEST_CODE = `let counter = 0
console.log("TS: Starting")

export async function main(name: string) {
  counter++
  console.log("TS: Hello " + name)
  return { greeting: "Hello " + name, count: counter }
}
`

const PYTHON_TEST_CODE = `import json

counter = 0

def main(name):
    global counter
    counter += 1
    print(f"PY: Hello {name}")
    return {"greeting": f"Hello {name}", "count": counter}
`

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

class TestClient {
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
		const url = `${SERVICE_URL}${endpoint}`
		console.log(`  Connecting to ${url}...`)

		return new Promise((resolve, reject) => {
			this.ws = new WebSocket(url)

			const timeout = setTimeout(() => {
				reject(new Error('Connection timeout'))
			}, 5000)

			this.ws.onopen = () => {
				clearTimeout(timeout)
				resolve()
			}

			this.ws.onerror = () => {
				clearTimeout(timeout)
				reject(new Error('Connection failed'))
			}

			this.ws.onmessage = (event) => {
				this.handleMessage(event.data as string)
			}

			this.ws.onclose = () => {
				console.log('  WebSocket closed')
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
		} catch (error) {
			console.error('  Failed to parse message:', error)
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

async function testTypescriptEndpoint(): Promise<boolean> {
	console.log('\n=== Testing TypeScript/Bun Endpoint (/typescript) ===')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')

		// Set breakpoint on line 6 (console.log inside main)
		await client.setBreakpoints('/test.ts', [6])
		console.log('  Breakpoints set')

		await client.configurationDone()
		await client.launch(TS_TEST_CODE, true, { name: 'World' })
		console.log('  Launched')

		await initP

		// Wait for stopped at breakpoint
		const stopped = await client.waitForEvent('stopped', 15000)
		console.log(`  Stopped at line ${stopped.body?.line}, reason: ${stopped.body?.reason}`)

		// Check output at breakpoint
		const outputAtBp = client.getOutput()
		console.log(`  Output at breakpoint: ${JSON.stringify(outputAtBp)}`)

		const hasHelloAtBp = outputAtBp.some(o => o.includes('Hello'))
		if (hasHelloAtBp) {
			console.log('  ❌ FAIL: "Hello" should not be printed at breakpoint')
			return false
		}
		console.log('  ✓ Breakpoint stopped before "Hello" was printed')

		// Continue
		await client.continue_()
		console.log('  Continued')

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 5000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		// Check final output and result
		const finalOutput = client.getOutput()
		console.log(`  Final output: ${JSON.stringify(finalOutput)}`)

		const result = client.getResult()
		console.log(`  Result: ${JSON.stringify(result)}`)

		// Verify
		const hasStarting = finalOutput.some(o => o.includes('Starting'))
		const hasHello = finalOutput.some(o => o.includes('Hello World'))
		const hasCorrectResult = result && (result as any).greeting === 'Hello World'

		if (!hasStarting) {
			console.log('  ❌ FAIL: Missing "Starting" output')
			return false
		}
		if (!hasHello) {
			console.log('  ❌ FAIL: Missing "Hello World" output')
			return false
		}
		if (!hasCorrectResult) {
			console.log('  ❌ FAIL: Incorrect result')
			return false
		}

		console.log('  ✓ All TypeScript tests passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testBunEndpointAlias(): Promise<boolean> {
	console.log('\n=== Testing Bun Endpoint Alias (/bun) ===')
	const client = new TestClient()

	try {
		await client.connect('/bun')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()
		await client.launch(TS_TEST_CODE, true, { name: 'Bun' })
		console.log('  Launched')

		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		const result = client.getResult()
		console.log(`  Result: ${JSON.stringify(result)}`)

		if (!result || (result as any).greeting !== 'Hello Bun') {
			console.log('  ❌ FAIL: Incorrect result')
			return false
		}

		console.log('  ✓ Bun alias endpoint works!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testPythonEndpoint(): Promise<boolean> {
	console.log('\n=== Testing Python Endpoint (/python) ===')
	const client = new TestClient()

	try {
		await client.connect('/python')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')

		// Set breakpoint on line 7 (print inside main)
		await client.setBreakpoints('/test.py', [7])
		console.log('  Breakpoints set')

		await client.configurationDone()
		await client.launch(PYTHON_TEST_CODE, true, { name: 'World' })
		console.log('  Launched')

		await initP

		// Wait for stopped at breakpoint
		const stopped = await client.waitForEvent('stopped', 15000)
		console.log(`  Stopped at line ${stopped.body?.line}, reason: ${stopped.body?.reason}`)

		// Continue
		await client.continue_()
		console.log('  Continued')

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		// Check final output and result
		const finalOutput = client.getOutput()
		console.log(`  Final output: ${JSON.stringify(finalOutput)}`)

		const result = client.getResult()
		console.log(`  Result: ${JSON.stringify(result)}`)

		// Verify
		const hasHello = finalOutput.some(o => o.includes('Hello World'))
		const hasCorrectResult = result && (result as any).greeting === 'Hello World'

		if (!hasHello) {
			console.log('  ❌ FAIL: Missing "Hello World" output')
			return false
		}
		if (!hasCorrectResult) {
			console.log('  ❌ FAIL: Incorrect result')
			return false
		}

		console.log('  ✓ All Python tests passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testHealthEndpoint(): Promise<boolean> {
	console.log('\n=== Testing Health Endpoint ===')

	try {
		const response = await fetch(`http://localhost:5679/health`)
		const data = await response.json()

		console.log(`  Health response: ${JSON.stringify(data)}`)

		if (data.status !== 'ok') {
			console.log('  ❌ FAIL: Health status not ok')
			return false
		}

		if (!data.endpoints || !data.endpoints.includes('/python') || !data.endpoints.includes('/typescript')) {
			console.log('  ❌ FAIL: Missing endpoints in health response')
			return false
		}

		console.log('  ✓ Health endpoint works!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	}
}

async function main() {
	console.log('='.repeat(60))
	console.log('DAP Debug Service Tests')
	console.log('='.repeat(60))

	let passed = 0
	let failed = 0

	// Test health endpoint
	if (await testHealthEndpoint()) {
		passed++
	} else {
		failed++
	}

	// Test TypeScript endpoint with breakpoints
	if (await testTypescriptEndpoint()) {
		passed++
	} else {
		failed++
	}

	// Test Bun alias endpoint
	if (await testBunEndpointAlias()) {
		passed++
	} else {
		failed++
	}

	// Test Python endpoint
	if (await testPythonEndpoint()) {
		passed++
	} else {
		failed++
	}

	// Summary
	console.log('\n' + '='.repeat(60))
	console.log(`TEST SUMMARY: ${passed} passed, ${failed} failed`)
	console.log('='.repeat(60))

	if (failed > 0) {
		console.log('\nNote: Python endpoint requires websockets package.')
		console.log('Run: pip install websockets')
		process.exit(1)
	} else {
		console.log('\n✓ All tests passed!')
		process.exit(0)
	}
}

main().catch((error) => {
	console.error('Test runner failed:', error)
	process.exit(1)
})
