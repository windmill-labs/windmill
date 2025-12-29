#!/usr/bin/env bun
/**
 * Test script for the Bun DAP WebSocket server.
 * This script connects to the server and tests breakpoint functionality.
 *
 * Run the server first:
 *    bun run dap_websocket_server_bun.ts
 *
 * Then run this test:
 *    bun run test_dap_server_bun.ts
 */

// Test TypeScript script with known breakpoints
const TEST_SCRIPT = `
const x = 1;
const y = 2;
const z = x + y;
console.log(\`Result: \${z}\`);
const w = z * 2;
console.log(\`Final: \${w}\`);
`

// Line numbers where we want to set breakpoints (1-indexed)
const BREAKPOINT_LINES = [3, 5] // z = x + y, w = z * 2

// Test script with main() function (Windmill style)
const TEST_SCRIPT_WITH_MAIN = `
export async function main(x: string, count: number = 1): Promise<string> {
    console.log(\`Starting with x=\${x}, count=\${count}\`);
    const result = x.repeat(count);
    console.log(\`Result: \${result}\`);
    return result;
}
`

// Breakpoints for the main() test: lines 3 and 4 (inside main function)
const MAIN_BREAKPOINT_LINES = [3, 4]

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

interface DAPEvent {
	event: string
	body?: Record<string, unknown>
}

class DAPTestClient {
	private url: string
	private ws: WebSocket | null = null
	private seq = 1
	private pendingRequests = new Map<number, { resolve: (value: DAPMessage) => void; reject: (error: Error) => void }>()
	private events: DAPEvent[] = []
	private stoppedEvents: DAPEvent[] = []
	private messageHandler: ((data: string) => void) | null = null

	constructor(url = 'ws://localhost:5680') {
		this.url = url
	}

	async connect(): Promise<void> {
		console.log(`Connecting to ${this.url}...`)

		return new Promise((resolve, reject) => {
			this.ws = new WebSocket(this.url)

			this.ws.onopen = () => {
				console.log('Connected!')
				resolve()
			}

			this.ws.onerror = (error) => {
				console.error('WebSocket error:', error)
				reject(new Error('WebSocket connection failed'))
			}

			this.ws.onmessage = (event) => {
				this.handleMessage(event.data as string)
			}

			this.ws.onclose = () => {
				console.log('WebSocket closed')
			}
		})
	}

	async disconnect(): Promise<void> {
		if (this.ws) {
			this.ws.close()
			this.ws = null
		}
	}

	private handleMessage(data: string): void {
		try {
			const msg = JSON.parse(data) as DAPMessage
			console.log(`<-- Received: ${JSON.stringify(msg, null, 2)}`)

			if (msg.type === 'response') {
				const reqSeq = msg.request_seq
				if (reqSeq !== undefined && this.pendingRequests.has(reqSeq)) {
					const pending = this.pendingRequests.get(reqSeq)!
					this.pendingRequests.delete(reqSeq)
					pending.resolve(msg)
				}
			} else if (msg.type === 'event') {
				this.events.push({ event: msg.event!, body: msg.body })
				if (msg.event === 'stopped') {
					this.stoppedEvents.push({ event: msg.event, body: msg.body })
				}
			}

			if (this.messageHandler) {
				this.messageHandler(data)
			}
		} catch (error) {
			console.error('Failed to parse message:', error)
		}
	}

	private async sendRequest(command: string, args?: Record<string, unknown>): Promise<DAPMessage> {
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

		console.log(`--> Sending: ${JSON.stringify(request, null, 2)}`)

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pendingRequests.delete(seq)
				reject(new Error(`Request timeout: ${command}`))
			}, 10000)

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

	async initialize(): Promise<DAPMessage> {
		return this.sendRequest('initialize', {
			clientID: 'test',
			clientName: 'DAP Test Client',
			adapterID: 'bun',
			pathFormat: 'path',
			linesStartAt1: true,
			columnsStartAt1: true
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

	async launch(code: string, cwd = '/tmp', callMain = false, args: Record<string, unknown> = {}): Promise<DAPMessage> {
		return this.sendRequest('launch', {
			code,
			cwd,
			callMain,
			args
		})
	}

	async continue_(): Promise<DAPMessage> {
		return this.sendRequest('continue', { threadId: 1 })
	}

	async getStackTrace(): Promise<DAPMessage> {
		return this.sendRequest('stackTrace', {
			threadId: 1,
			startFrame: 0,
			levels: 20
		})
	}

	async getScopes(frameId: number): Promise<DAPMessage> {
		return this.sendRequest('scopes', { frameId })
	}

	async getVariables(varRef: number): Promise<DAPMessage> {
		return this.sendRequest('variables', { variablesReference: varRef })
	}

	async terminate(): Promise<DAPMessage> {
		return this.sendRequest('terminate')
	}

	async waitForStopped(timeout = 5000): Promise<DAPEvent> {
		const start = this.stoppedEvents.length
		const startTime = Date.now()

		while (Date.now() - startTime < timeout) {
			if (this.stoppedEvents.length > start) {
				return this.stoppedEvents[this.stoppedEvents.length - 1]
			}
			await new Promise((resolve) => setTimeout(resolve, 100))
		}

		throw new Error('Timeout waiting for stopped event')
	}

	async waitForEvent(eventName: string, timeout = 5000): Promise<DAPEvent> {
		const start = this.events.length
		const startTime = Date.now()

		while (Date.now() - startTime < timeout) {
			for (let i = start; i < this.events.length; i++) {
				if (this.events[i].event === eventName) {
					return this.events[i]
				}
			}
			await new Promise((resolve) => setTimeout(resolve, 100))
		}

		throw new Error(`Timeout waiting for ${eventName} event`)
	}
}

async function runTest(): Promise<void> {
	const client = new DAPTestClient()

	try {
		await client.connect()
		await new Promise((resolve) => setTimeout(resolve, 100))

		// 1. Initialize
		console.log('\n=== STEP 1: Initialize ===')
		let response = await client.initialize()
		if (!response.success) throw new Error(`Initialize failed: ${JSON.stringify(response)}`)
		console.log('Initialize: OK')

		// Wait for initialized event
		await new Promise((resolve) => setTimeout(resolve, 500))

		// 2. Set breakpoints
		console.log('\n=== STEP 2: Set Breakpoints ===')
		response = await client.setBreakpoints('/tmp/script.ts', BREAKPOINT_LINES)
		if (!response.success) throw new Error(`setBreakpoints failed: ${JSON.stringify(response)}`)
		const breakpoints = (response.body?.breakpoints as unknown[]) || []
		console.log(`Breakpoints set: ${JSON.stringify(breakpoints)}`)
		if (breakpoints.length !== BREAKPOINT_LINES.length) {
			console.log('WARNING: Wrong number of breakpoints')
		}

		// 3. Configuration done
		console.log('\n=== STEP 3: Configuration Done ===')
		response = await client.configurationDone()
		if (!response.success) throw new Error(`configurationDone failed: ${JSON.stringify(response)}`)
		console.log('Configuration done: OK')

		// 4. Launch
		console.log('\n=== STEP 4: Launch ===')
		response = await client.launch(TEST_SCRIPT)
		if (!response.success) throw new Error(`launch failed: ${JSON.stringify(response)}`)
		console.log('Launch: OK')

		// 5. Wait for first breakpoint
		console.log('\n=== STEP 5: Wait for First Breakpoint ===')
		try {
			const stopped = await client.waitForStopped(10000)
			console.log(`Stopped at: ${JSON.stringify(stopped)}`)

			const reason = stopped.body?.reason
			console.log(`Stop reason: ${reason}`)

			if (reason === 'breakpoint') {
				console.log('SUCCESS: Hit first breakpoint!')
			} else {
				console.log(`WARNING: Stopped for reason '${reason}', not 'breakpoint'`)
			}

			// Get stack trace
			console.log('\n=== STEP 6: Get Stack Trace ===')
			const stackResponse = await client.getStackTrace()
			const frames = (stackResponse.body?.stackFrames as Array<{ line: number; id: number; name: string }>) || []
			if (frames.length > 0) {
				const currentLine = frames[0].line
				console.log(`Current line: ${currentLine}`)
				if (BREAKPOINT_LINES.includes(currentLine)) {
					console.log(`SUCCESS: Stopped at expected line ${currentLine}`)
				} else {
					console.log(`WARNING: Stopped at line ${currentLine}, expected one of ${BREAKPOINT_LINES}`)
				}
			}

			// Get variables
			console.log('\n=== STEP 7: Get Variables ===')
			const scopesResponse = await client.getScopes(frames[0]?.id || 1)
			const scopes = (scopesResponse.body?.scopes as Array<{ name: string; variablesReference: number }>) || []
			console.log(`Scopes: ${scopes.map((s) => s.name).join(', ')}`)

			if (scopes.length > 0) {
				const varsResponse = await client.getVariables(scopes[0].variablesReference)
				const variables = (varsResponse.body?.variables as Array<{ name: string; value: string }>) || []
				console.log(`Local variables: ${variables.map((v) => `${v.name}=${v.value}`).join(', ')}`)
			}

			// Continue to next breakpoint
			console.log('\n=== STEP 8: Continue to Next Breakpoint ===')
			await client.continue_()

			try {
				const stopped2 = await client.waitForStopped(5000)
				console.log(`Stopped again at: ${JSON.stringify(stopped2)}`)

				const stackResponse2 = await client.getStackTrace()
				const frames2 = (stackResponse2.body?.stackFrames as Array<{ line: number }>) || []
				if (frames2.length > 0) {
					const currentLine = frames2[0].line
					console.log(`Current line: ${currentLine}`)

					if (BREAKPOINT_LINES.includes(currentLine)) {
						console.log(`SUCCESS: Hit second breakpoint at line ${currentLine}!`)
					} else {
						console.log(`INFO: Stopped at line ${currentLine}`)
					}
				}

				// Continue to end
				console.log('\n=== STEP 9: Continue to End ===')
				await client.continue_()

				// Wait for terminated event
				try {
					await client.waitForEvent('terminated', 5000)
					console.log('Script terminated normally')
				} catch {
					console.log('Timeout waiting for terminated event')
				}
			} catch {
				console.log('No second breakpoint hit - script may have ended')
			}
		} catch (error) {
			console.log('ERROR: No breakpoint was hit!')
			console.log('The script ran without stopping at breakpoints.')
			console.log('\nCheck server logs for:')
			console.log('  - Inspector connection messages')
			console.log('  - Debugger.scriptParsed events')
			console.log('  - Debugger.paused events')
		}

		// Terminate
		console.log('\n=== STEP 10: Terminate ===')
		try {
			await client.terminate()
			console.log('Terminated: OK')
		} catch (error) {
			console.log(`Terminate error (may be expected): ${error}`)
		}
	} catch (error) {
		console.error(`\nERROR: ${error}`)
		console.error(error)
	} finally {
		await client.disconnect()
		console.log('\n=== TEST COMPLETE ===')
	}
}

async function runMainTest(): Promise<void> {
	const client = new DAPTestClient()

	try {
		await client.connect()
		await new Promise((resolve) => setTimeout(resolve, 100))

		// 1. Initialize
		console.log('\n=== MAIN TEST: Initialize ===')
		let response = await client.initialize()
		if (!response.success) throw new Error(`Initialize failed: ${JSON.stringify(response)}`)
		console.log('Initialize: OK')
		await new Promise((resolve) => setTimeout(resolve, 500))

		// 2. Set breakpoints inside main()
		console.log('\n=== MAIN TEST: Set Breakpoints ===')
		response = await client.setBreakpoints('/tmp/script.ts', MAIN_BREAKPOINT_LINES)
		if (!response.success) throw new Error(`setBreakpoints failed: ${JSON.stringify(response)}`)
		console.log(`Breakpoints set at lines: ${MAIN_BREAKPOINT_LINES}`)

		// 3. Configuration done
		console.log('\n=== MAIN TEST: Configuration Done ===')
		response = await client.configurationDone()
		if (!response.success) throw new Error(`configurationDone failed: ${JSON.stringify(response)}`)

		// 4. Launch with callMain=True and args
		console.log('\n=== MAIN TEST: Launch with callMain=true ===')
		const testArgs = { x: 'hello', count: 3 }
		response = await client.launch(TEST_SCRIPT_WITH_MAIN, '/tmp', true, testArgs)
		if (!response.success) throw new Error(`launch failed: ${JSON.stringify(response)}`)
		console.log(`Launch with args ${JSON.stringify(testArgs)}: OK`)

		// 5. Wait for breakpoint inside main()
		console.log('\n=== MAIN TEST: Wait for Breakpoint in main() ===')
		try {
			const stopped = await client.waitForStopped(10000)
			const reason = stopped.body?.reason
			console.log(`Stopped! Reason: ${reason}`)

			// Get stack trace
			const stackResponse = await client.getStackTrace()
			const frames = (stackResponse.body?.stackFrames as Array<{ line: number; id: number; name: string }>) || []
			if (frames.length > 0) {
				const currentLine = frames[0].line
				const funcName = frames[0].name
				console.log(`Current location: ${funcName}() at line ${currentLine}`)

				if (funcName === 'main') {
					console.log('SUCCESS: Stopped inside main() function!')
				} else {
					console.log(`INFO: Stopped in function '${funcName}'`)
				}

				// Get local variables to verify args were passed
				const scopesResponse = await client.getScopes(frames[0].id)
				const scopes = (scopesResponse.body?.scopes as Array<{ name: string; variablesReference: number }>) || []
				if (scopes.length > 0) {
					const varsResponse = await client.getVariables(scopes[0].variablesReference)
					const variables = (varsResponse.body?.variables as Array<{ name: string; value: string }>) || []
					const varDict: Record<string, string> = {}
					for (const v of variables) {
						varDict[v.name] = v.value
					}
					console.log(`Variables: ${JSON.stringify(varDict)}`)

					// Check if our args are present
					if ('x' in varDict && 'count' in varDict) {
						console.log(`SUCCESS: Args passed correctly! x=${varDict['x']}, count=${varDict['count']}`)
					}
				}
			}

			// Continue to end
			console.log('\n=== MAIN TEST: Continue to End ===')
			await client.continue_()

			// May hit another breakpoint or end
			try {
				await client.waitForStopped(2000)
				console.log('Hit another breakpoint')
				await client.continue_()
			} catch {
				// Expected if no more breakpoints
			}

			// Wait for output/terminated
			await new Promise((resolve) => setTimeout(resolve, 1000))
		} catch (error) {
			console.log(`ERROR: No breakpoint hit inside main()! ${error}`)
		}

		// Terminate
		console.log('\n=== MAIN TEST: Terminate ===')
		try {
			await client.terminate()
			console.log('Terminated: OK')
		} catch (error) {
			console.log(`Terminate: ${error}`)
		}
	} catch (error) {
		console.error(`\nERROR: ${error}`)
		console.error(error)
	} finally {
		await client.disconnect()
		console.log('\n=== MAIN TEST COMPLETE ===')
	}
}

// Parse command line arguments
const cliArgs = process.argv.slice(2)
const runMainOnly = cliArgs.includes('--main')
const runAll = cliArgs.includes('--all')

if (runMainOnly) {
	runMainTest()
} else if (runAll) {
	runTest().then(() => {
		console.log('\n' + '='.repeat(60) + '\n')
		return runMainTest()
	})
} else {
	runTest()
}
