#!/usr/bin/env bun
/**
 * Test script for the Bun DAP WebSocket server.
 *
 * Tests breakpoints on every line, variable inspection, and console output.
 *
 * Run the server first:
 *    bun run dap_websocket_server_bun.ts
 *
 * Then run this test:
 *    bun run test_dap_server_bun.ts
 */

// Test script with module-level code and a main function
const TEST_SCRIPT = `let foo = 42
console.log("A")  // Line 1 - module level
console.log("B")  // Line 2 - module level

export async function main(x: string) {
  let qwe = "xaweqw"
  let foobar = 312312
  console.log("A")
  console.log("B")
  return x
}`

// All executable lines (skipping empty line 4 and function signature line 5)
// Line numbers are 1-indexed as they appear in the source
const BREAKPOINT_LINES = [1, 2, 3, 6, 7, 8, 9, 10]

// Expected variables at each line (variable should be visible AFTER the line executes)
// For breakpoints, we're stopped BEFORE the line executes, so:
// - Line 1: `let foo = 42` - foo not yet assigned
// - Line 2: after foo assignment, foo=42 (NOTE: module-level `let` not captured by Bun inspector)
// - Line 3: still foo=42 (NOTE: module-level `let` not captured by Bun inspector)
// - Line 6: inside main, x is parameter
// - Line 7: x, qwe assigned
// - Line 8: x, qwe, foobar assigned
// - Line 9: x, qwe, foobar still there
// - Line 10: x, qwe, foobar, about to return
//
// KNOWN LIMITATION: Bun's WebKit inspector does not expose module-level `let`/`const`
// declarations in the scope chain. Only variables inside functions are visible.
const EXPECTED_VARIABLES: Record<number, string[]> = {
	1: [], // stopped before foo assignment
	2: [], // NOTE: foo not visible in Bun's inspector for module-level let
	3: [], // NOTE: foo not visible in Bun's inspector for module-level let
	6: ['x'], // inside main, x is parameter
	7: ['x', 'qwe'], // qwe assigned
	8: ['x', 'qwe', 'foobar'], // foobar assigned
	9: ['x', 'qwe', 'foobar'], // same
	10: ['x', 'qwe', 'foobar'] // same
}

// Expected console output order
const EXPECTED_LOGS = ['A', 'B', 'A', 'B']

// Test args
const TEST_ARGS = { x: 'foobar' }

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

interface Variable {
	name: string
	value: string
	type?: string
	variablesReference: number
}

interface StackFrame {
	id: number
	name: string
	line: number
	column: number
	source?: { path: string; name?: string }
}

interface Scope {
	name: string
	variablesReference: number
	expensive: boolean
}

class DAPTestClient {
	private url: string
	private ws: WebSocket | null = null
	private seq = 1
	private pendingRequests = new Map<
		number,
		{ resolve: (value: DAPMessage) => void; reject: (error: Error) => void }
	>()
	private events: DAPMessage[] = []
	private output: string[] = []
	private eventHandlers: Map<string, ((event: DAPMessage) => void)[]> = new Map()
	private terminatedResult: unknown = undefined

	constructor(url = 'ws://localhost:5680') {
		this.url = url
	}

	async connect(): Promise<void> {
		console.log(`[TEST] Connecting to ${this.url}...`)

		return new Promise((resolve, reject) => {
			this.ws = new WebSocket(this.url)

			this.ws.onopen = () => {
				console.log('[TEST] Connected!')
				resolve()
			}

			this.ws.onerror = (error) => {
				console.error('[TEST] WebSocket error:', error)
				reject(new Error('WebSocket connection failed'))
			}

			this.ws.onmessage = (event) => {
				this.handleMessage(event.data as string)
			}

			this.ws.onclose = () => {
				console.log('[TEST] WebSocket closed')
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
				const reqSeq = msg.request_seq
				if (reqSeq !== undefined && this.pendingRequests.has(reqSeq)) {
					const pending = this.pendingRequests.get(reqSeq)!
					this.pendingRequests.delete(reqSeq)
					if (msg.success) {
						pending.resolve(msg)
					} else {
						pending.reject(new Error(msg.message || 'Request failed'))
					}
				}
			} else if (msg.type === 'event') {
				console.log(`[TEST] Event: ${msg.event}`, JSON.stringify(msg.body))
				this.events.push(msg)

				// Capture output
				if (msg.event === 'output' && msg.body?.output) {
					const outputStr = String(msg.body.output).trim()
					if (outputStr) {
						this.output.push(outputStr)
					}
				}

				// Capture result from terminated event
				if (msg.event === 'terminated' && msg.body?.result !== undefined) {
					this.terminatedResult = msg.body.result
				}

				// Notify handlers
				const handlers = this.eventHandlers.get(msg.event!) || []
				for (const handler of handlers) {
					handler(msg)
				}
			}
		} catch (error) {
			console.error('[TEST] Failed to parse message:', error)
		}
	}

	private async sendRequest(
		command: string,
		args?: Record<string, unknown>
	): Promise<DAPMessage> {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			throw new Error('Not connected')
		}

		const seq = this.seq++
		const request: DAPMessage = {
			seq,
			type: 'request',
			command,
			arguments: args
		}

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

	waitForEvent(eventName: string, timeout: number = 10000): Promise<DAPMessage> {
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

	async initialize(): Promise<DAPMessage> {
		return this.sendRequest('initialize', {
			clientID: 'test',
			clientName: 'DAP Test Client',
			adapterID: 'bun',
			pathFormat: 'path',
			linesStartAt1: true,
			columnsStartAt1: true,
			supportsVariableType: true
		})
	}

	async setBreakpoints(path: string, lines: number[]): Promise<DAPMessage> {
		return this.sendRequest('setBreakpoints', {
			source: { path },
			breakpoints: lines.map((line) => ({ line }))
		})
	}

	async configurationDone(): Promise<DAPMessage> {
		return this.sendRequest('configurationDone')
	}

	async launch(
		code: string,
		callMain = false,
		args: Record<string, unknown> = {}
	): Promise<DAPMessage> {
		return this.sendRequest('launch', {
			code,
			callMain,
			args
		})
	}

	async continue_(): Promise<DAPMessage> {
		return this.sendRequest('continue', { threadId: 1 })
	}

	async next(): Promise<DAPMessage> {
		return this.sendRequest('next', { threadId: 1 })
	}

	async getStackTrace(): Promise<StackFrame[]> {
		const response = await this.sendRequest('stackTrace', {
			threadId: 1,
			startFrame: 0,
			levels: 20
		})
		return (response.body?.stackFrames as StackFrame[]) || []
	}

	async getScopes(frameId: number): Promise<Scope[]> {
		const response = await this.sendRequest('scopes', { frameId })
		return (response.body?.scopes as Scope[]) || []
	}

	async getVariables(variablesReference: number): Promise<Variable[]> {
		const response = await this.sendRequest('variables', { variablesReference })
		return (response.body?.variables as Variable[]) || []
	}

	async terminate(): Promise<DAPMessage | null> {
		// Check if still connected before sending terminate
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			console.log('[TEST] WebSocket not open, skipping terminate (script may have already ended)')
			return null
		}
		return this.sendRequest('terminate')
	}

	isConnected(): boolean {
		return this.ws !== null && this.ws.readyState === WebSocket.OPEN
	}

	getOutput(): string[] {
		return this.output
	}

	clearOutput(): void {
		this.output = []
	}

	getResult(): unknown {
		return this.terminatedResult
	}
}

// Test results tracking
interface TestResult {
	test: string
	passed: boolean
	error?: string
	details?: string
}

async function runComprehensiveTest(): Promise<void> {
	const client = new DAPTestClient()
	const results: TestResult[] = []
	let passed = 0
	let failed = 0

	function assert(condition: boolean, testName: string, details?: string): void {
		if (condition) {
			console.log(`[PASS] ${testName}`)
			passed++
			results.push({ test: testName, passed: true, details })
		} else {
			console.log(`[FAIL] ${testName}${details ? ': ' + details : ''}`)
			failed++
			results.push({ test: testName, passed: false, error: details })
		}
	}

	try {
		// Connect
		console.log('\n=== Connecting to DAP Server ===')
		await client.connect()
		assert(true, 'Connect to DAP server')

		// Initialize
		console.log('\n=== Initialize ===')
		const initResponse = await client.initialize()
		assert(initResponse.success === true, 'Initialize session')

		// Set up initialized event handler before setting breakpoints
		const initializedPromise = client.waitForEvent('initialized')

		// Set breakpoints on all executable lines
		console.log('\n=== Set Breakpoints ===')
		console.log(`[TEST] Setting breakpoints on lines: ${BREAKPOINT_LINES}`)
		const bpResponse = await client.setBreakpoints('/tmp/test_script.ts', BREAKPOINT_LINES)
		assert(bpResponse.success === true, 'Set breakpoints request')
		const breakpoints = bpResponse.body?.breakpoints as Array<{
			verified: boolean
			line: number
		}>
		assert(
			breakpoints.length === BREAKPOINT_LINES.length,
			`Breakpoint count (expected ${BREAKPOINT_LINES.length}, got ${breakpoints.length})`
		)

		// Configuration done
		console.log('\n=== Configuration Done ===')
		await client.configurationDone()
		assert(true, 'Configuration done')

		// Launch with callMain=true and args
		console.log('\n=== Launch Script ===')
		console.log(`[TEST] Launching with callMain=true, args=${JSON.stringify(TEST_ARGS)}`)
		await client.launch(TEST_SCRIPT, true, TEST_ARGS)
		assert(true, 'Launch script')

		// Wait for initialized event
		await initializedPromise
		assert(true, 'Received initialized event')

		// Track breakpoints hit and variables at each
		const breakpointsHit: number[] = []
		const variablesAtBreakpoints: Map<number, string[]> = new Map()
		const variableValuesAtBreakpoints: Map<number, Map<string, string>> = new Map()

		// Process breakpoints
		console.log('\n=== Processing Breakpoints ===')
		let iteration = 0
		const maxIterations = BREAKPOINT_LINES.length + 5

		while (iteration < maxIterations) {
			iteration++

			// Wait for stopped event
			console.log(`\n[TEST] Waiting for stopped event (iteration ${iteration})...`)
			let stoppedEvent: DAPMessage
			try {
				stoppedEvent = await client.waitForEvent('stopped', 15000)
			} catch {
				console.log('[TEST] No more stopped events (likely script completed)')
				break
			}

			const line = stoppedEvent.body?.line as number | undefined
			const reason = stoppedEvent.body?.reason as string
			console.log(`[TEST] Stopped at line ${line}, reason: ${reason}`)

			if (line !== undefined) {
				breakpointsHit.push(line)
			}

			// Get stack trace
			const frames = await client.getStackTrace()
			console.log(`[TEST] Stack trace: ${frames.length} frames`)
			for (const frame of frames) {
				console.log(`[TEST]   Frame ${frame.id}: ${frame.name} at line ${frame.line}`)
			}

			if (frames.length > 0) {
				// Get scopes
				const scopes = await client.getScopes(frames[0].id)
				console.log(`[TEST] Scopes: ${scopes.length}`)

				// Get variables from all non-expensive scopes
				const allVars: string[] = []
				const varValues: Map<string, string> = new Map()
				for (const scope of scopes) {
					if (!scope.expensive) {
						console.log(
							`[TEST]   Scope: ${scope.name} (ref: ${scope.variablesReference})`
						)
						const vars = await client.getVariables(scope.variablesReference)
						console.log(`[TEST]   Variables in ${scope.name}: ${vars.length}`)
						for (const v of vars) {
							console.log(`[TEST]     ${v.name} = ${v.value} (${v.type})`)
							allVars.push(v.name)
							varValues.set(v.name, v.value)
						}
					}
				}

				if (line !== undefined) {
					variablesAtBreakpoints.set(line, allVars)
					variableValuesAtBreakpoints.set(line, varValues)
				}
			}

			// Continue to next breakpoint
			console.log('[TEST] Continuing...')
			await client.continue_()
		}

		// Wait for terminated event or timeout
		console.log('\n[TEST] Waiting for terminated event...')
		try {
			await client.waitForEvent('terminated', 5000)
			console.log('[TEST] Script terminated')
		} catch {
			console.log('[TEST] Timeout waiting for terminated event (script may have already ended)')
		}

		// Wait a bit more for final output to flush
		await new Promise((resolve) => setTimeout(resolve, 1000))

		// === VERIFICATION ===
		console.log('\n' + '='.repeat(60))
		console.log('VERIFICATION')
		console.log('='.repeat(60))

		// Verify breakpoints were hit
		console.log('\n=== Breakpoint Analysis ===')
		console.log(`[TEST] Breakpoints hit: ${breakpointsHit}`)
		console.log(`[TEST] Expected breakpoints: ${BREAKPOINT_LINES}`)

		for (const expectedLine of BREAKPOINT_LINES) {
			const hitCount = breakpointsHit.filter((l) => l === expectedLine).length
			assert(hitCount > 0, `Breakpoint at line ${expectedLine} was hit`)
		}

		// Verify variables at each breakpoint
		console.log('\n=== Variable Analysis ===')
		for (const [line, expectedVars] of Object.entries(EXPECTED_VARIABLES)) {
			const lineNum = parseInt(line)
			const actualVars = variablesAtBreakpoints.get(lineNum) || []
			console.log(
				`[TEST] Line ${lineNum}: expected [${expectedVars}], got [${actualVars}]`
			)

			for (const expectedVar of expectedVars) {
				assert(
					actualVars.includes(expectedVar),
					`Variable '${expectedVar}' visible at line ${lineNum}`,
					`Available: ${actualVars.join(', ')}`
				)
			}
		}

		// Verify variable x has correct value inside main() (line 6 or later)
		console.log('\n=== Variable Value Check ===')
		const varsAtLine6 = variableValuesAtBreakpoints.get(6)
		if (varsAtLine6) {
			const xValue = varsAtLine6.get('x')
			console.log(`[TEST] Variable x at line 6: ${xValue}`)
			assert(
				xValue === '"foobar"',
				'Variable x has correct value "foobar"',
				`Got: ${xValue}`
			)
		} else {
			assert(false, 'Variable x has correct value "foobar"', 'No variables at line 6')
		}

		// Verify console output
		console.log('\n=== Console Output Analysis ===')
		const output = client.getOutput()
		console.log(`[TEST] Captured output: ${JSON.stringify(output)}`)
		console.log(`[TEST] Expected output: ${JSON.stringify(EXPECTED_LOGS)}`)

		// Filter out result output and any stderr noise
		const consoleOutput = output.filter(
			(o) => !o.startsWith('__WINDMILL_RESULT__') && !o.includes('[DEBUG]')
		)
		assert(
			consoleOutput.length >= EXPECTED_LOGS.length,
			`Console output count (expected >= ${EXPECTED_LOGS.length}, got ${consoleOutput.length})`
		)

		for (let i = 0; i < EXPECTED_LOGS.length; i++) {
			assert(
				consoleOutput[i] === EXPECTED_LOGS[i],
				`Console output[${i}] is "${EXPECTED_LOGS[i]}"`,
				`Got: "${consoleOutput[i]}"`
			)
		}

		// Terminate and get result
		console.log('\n=== Terminate ===')
		const terminateResult = await client.terminate()
		if (terminateResult !== null) {
			assert(true, 'Terminate session')
		} else {
			assert(true, 'Script already terminated naturally')
		}

		// Wait for terminated event with result
		await new Promise((resolve) => setTimeout(resolve, 500))

		// Check for return value from terminated event
		console.log('\n=== Return Value Analysis ===')
		const result = client.getResult()
		console.log(`[TEST] Result from terminated event: ${JSON.stringify(result)}`)
		assert(result === 'foobar', 'Return value is "foobar"', `Got: ${JSON.stringify(result)}`)
	} catch (error) {
		console.error('[TEST] Error:', error)
		failed++
		results.push({ test: 'Unexpected error', passed: false, error: String(error) })
	} finally {
		client.disconnect()
	}

	// Summary
	console.log('\n' + '='.repeat(60))
	console.log(`TEST SUMMARY: ${passed} passed, ${failed} failed`)
	console.log('='.repeat(60))

	if (failed > 0) {
		console.log('\nFailed tests:')
		for (const r of results.filter((r) => !r.passed)) {
			console.log(`  - ${r.test}: ${r.error || 'Unknown error'}`)
		}
		process.exit(1)
	} else {
		console.log('\n✓ All tests passed!')
		process.exit(0)
	}
}

// Run the test
runComprehensiveTest().catch((error) => {
	console.error('Test runner failed:', error)
	process.exit(1)
})
