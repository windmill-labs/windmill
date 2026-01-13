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
 *
 * To test with nsjail enabled:
 *    bun run dap_debug_service.ts --nsjail --nsjail-config /path/to/config.cfg
 *    bun run test_debug_service.ts --nsjail
 *
 * To test automatic dependency installation (autoinstall):
 *    bun run dap_debug_service.ts --windmill /path/to/windmill
 *    bun run test_debug_service.ts --test-autoinstall
 */

import { spawn } from 'bun'

const SERVICE_URL = 'ws://localhost:3003'
const TEST_NSJAIL = process.argv.includes('--nsjail')
const TEST_AUTOINSTALL = process.argv.includes('--test-autoinstall')

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

const TS_ENV_TEST_CODE = `export async function main() {
  const workspace = process.env.WM_WORKSPACE || "unknown"
  const token = process.env.WM_TOKEN || "unknown"
  console.log("ENV: Workspace=" + workspace)
  return { workspace, token }
}
`

const PYTHON_ENV_TEST_CODE = `import os

def main():
    workspace = os.environ.get("WM_WORKSPACE", "unknown")
    token = os.environ.get("WM_TOKEN", "unknown")
    print(f"ENV: Workspace={workspace}")
    return {"workspace": workspace, "token": token}
`

// Test code for console.log with objects (should show object contents, not just "Object")
const TS_CONSOLE_OBJECT_CODE = `export async function main() {
  const obj = { foo: "bar", count: 42 }
  console.log(obj)
  console.log("Done")
  return obj
}
`

// Test code for variable panel display - object values should show properties, not just "Object"
const TS_VARIABLE_OBJECT_CODE = `export async function main() {
  const myObj = { name: "test", value: 123 }
  const myArr = [1, 2, 3]
  console.log("breakpoint here")
  return { myObj, myArr }
}
`

// Test code for Bun autoinstall - uses lodash which must be auto-installed
const TS_AUTOINSTALL_CODE = `import _ from "lodash"

export async function main(items: number[]) {
  const sum = _.sum(items)
  const max = _.max(items)
  console.log("Sum:", sum)
  console.log("Max:", max)
  return { sum, max }
}
`

// Test code for Python autoinstall - uses requests which must be auto-installed
const PYTHON_AUTOINSTALL_CODE = `import requests

def main():
    # Just verify that requests is importable and has expected attributes
    version = requests.__version__
    print(f"Requests version: {version}")
    return {"version": version, "imported": True}
`

// Test code for Bun versioned imports - uses number-to-words@1 syntax
const TS_VERSIONED_IMPORT_CODE = `import { toWords } from "number-to-words@1"

export async function main(num: number) {
  const words = toWords(num)
  console.log("Number in words:", words)
  return { num, words }
}
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

	async launch(code: string, callMain = false, args: Record<string, unknown> = {}, env: Record<string, string> = {}) {
		return this.sendRequest('launch', { code, callMain, args, env })
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

	async stackTrace(threadId: number = 1) {
		return this.sendRequest('stackTrace', { threadId })
	}

	async scopes(frameId: number) {
		return this.sendRequest('scopes', { frameId })
	}

	async variables(variablesReference: number) {
		return this.sendRequest('variables', { variablesReference })
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

async function testEnvVarsTypescript(): Promise<boolean> {
	console.log('\n=== Testing Environment Variables (TypeScript) ===')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()

		// Pass env vars in launch request
		await client.launch(TS_ENV_TEST_CODE, true, {}, {
			WM_WORKSPACE: 'test-workspace',
			WM_TOKEN: 'secret-token-123'
		})
		console.log('  Launched with env vars')

		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		const result = client.getResult() as { workspace: string; token: string } | undefined
		console.log(`  Result: ${JSON.stringify(result)}`)

		if (!result || result.workspace !== 'test-workspace' || result.token !== 'secret-token-123') {
			console.log('  ❌ FAIL: Env vars not passed correctly')
			return false
		}

		console.log('  ✓ TypeScript env vars test passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testEnvVarsPython(): Promise<boolean> {
	console.log('\n=== Testing Environment Variables (Python) ===')
	const client = new TestClient()

	try {
		await client.connect('/python')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.py', [])
		await client.configurationDone()

		// Pass env vars in launch request
		await client.launch(PYTHON_ENV_TEST_CODE, true, {}, {
			WM_WORKSPACE: 'py-workspace',
			WM_TOKEN: 'py-token-456'
		})
		console.log('  Launched with env vars')

		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		const result = client.getResult() as { workspace: string; token: string } | undefined
		console.log(`  Result: ${JSON.stringify(result)}`)

		if (!result || result.workspace !== 'py-workspace' || result.token !== 'py-token-456') {
			console.log('  ❌ FAIL: Env vars not passed correctly')
			return false
		}

		console.log('  ✓ Python env vars test passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testConsoleObjectOutput(): Promise<boolean> {
	console.log('\n=== Testing Console Object Output (TypeScript) ===')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()

		await client.launch(TS_CONSOLE_OBJECT_CODE, true, {})
		console.log('  Launched')

		await initP

		// Wait for termination
		try {
			await client.waitForEvent('terminated', 10000)
		} catch {}

		await new Promise(r => setTimeout(r, 500))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		// Check that the object output contains actual property values, not just "Object"
		const objectOutput = output.find(o => o.includes('foo'))
		if (!objectOutput) {
			console.log('  ❌ FAIL: Object should show property names like "foo"')
			return false
		}

		// Check that we see the property values
		if (!objectOutput.includes('bar') && !objectOutput.includes('"bar"')) {
			console.log('  ❌ FAIL: Object should show property values like "bar"')
			return false
		}

		// Make sure it's not just showing "Object" or "[object Object]"
		if (output.some(o => o.trim() === 'Object' || o.includes('[object Object]'))) {
			console.log('  ❌ FAIL: Should not just show "Object" or "[object Object]"')
			return false
		}

		console.log('  ✓ Console object output test passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	} finally {
		client.disconnect()
	}
}

async function testVariableObjectDisplay(): Promise<boolean> {
	console.log('\n=== Testing Variable Panel Object Display ===')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')

		// Set breakpoint on line 4 (console.log("breakpoint here"))
		await client.setBreakpoints('/test.ts', [4])
		console.log('  Breakpoint set on line 4')

		await client.configurationDone()
		await client.launch(TS_VARIABLE_OBJECT_CODE, true, {})
		console.log('  Launched')

		await initP

		// Wait for stopped at breakpoint
		const stopped = await client.waitForEvent('stopped', 15000)
		console.log(`  Stopped at line ${stopped.body?.line}, reason: ${stopped.body?.reason}`)

		// Get stack trace
		const stackTraceResponse = await client.stackTrace()
		const frames = stackTraceResponse.body?.stackFrames as Array<{ id: number; name: string }> | undefined
		if (!frames || frames.length === 0) {
			console.log('  ❌ FAIL: No stack frames returned')
			return false
		}
		console.log(`  Stack frames: ${frames.map(f => f.name).join(', ')}`)

		// Get scopes for the first frame
		const scopesResponse = await client.scopes(frames[0].id)
		const scopes = scopesResponse.body?.scopes as Array<{ name: string; variablesReference: number }> | undefined
		if (!scopes || scopes.length === 0) {
			console.log('  ❌ FAIL: No scopes returned')
			return false
		}
		console.log(`  Scopes: ${scopes.map(s => s.name).join(', ')}`)

		// Get variables from all scopes to find our local variables
		let foundVariables: Array<{ name: string; value: string; type: string }> = []
		for (const scope of scopes) {
			console.log(`  Checking scope: ${scope.name} (variablesReference: ${scope.variablesReference})`)
			const varsResponse = await client.variables(scope.variablesReference)
			const scopeVars = varsResponse.body?.variables as Array<{ name: string; value: string; type: string }> | undefined
			if (scopeVars && scopeVars.length > 0) {
				console.log(`    Found ${scopeVars.length} variables: ${scopeVars.map(v => v.name).join(', ')}`)
				// Look for our specific variables
				const myObj = scopeVars.find(v => v.name === 'myObj')
				const myArr = scopeVars.find(v => v.name === 'myArr')
				if (myObj || myArr) {
					foundVariables = scopeVars
					console.log(`    Found target variables in this scope!`)
					break
				}
			}
		}

		if (foundVariables.length === 0) {
			console.log('  ❌ FAIL: No relevant variables found in any scope')
			return false
		}

		const variables = foundVariables
		console.log(`  Variables: ${JSON.stringify(variables.map(v => ({ name: v.name, value: v.value })))}`)

		// Find myObj variable and check its value
		const myObjVar = variables.find(v => v.name === 'myObj')
		if (!myObjVar) {
			console.log('  ❌ FAIL: myObj variable not found')
			return false
		}

		// Check that myObj shows properties, not just "Object"
		const objValue = myObjVar.value
		console.log(`  myObj value: ${objValue}`)

		// It should NOT be just "Object" or "[object Object]"
		if (objValue === 'Object' || objValue === '[object Object]' || objValue === '[Object]') {
			console.log('  ❌ FAIL: myObj shows just "Object" instead of properties')
			return false
		}

		// It should contain property names or values
		if (!objValue.includes('name') && !objValue.includes('test') && !objValue.includes('value') && !objValue.includes('123')) {
			console.log('  ❌ FAIL: myObj should show properties like "name" or "value"')
			return false
		}

		// Find myArr variable and check its value
		const myArrVar = variables.find(v => v.name === 'myArr')
		if (myArrVar) {
			console.log(`  myArr value: ${myArrVar.value}`)
			// Array should show contents, not just "Array"
			if (myArrVar.value === 'Array' || myArrVar.value === '[object Array]') {
				console.log('  ❌ FAIL: myArr shows just "Array" instead of contents')
				return false
			}
		}

		// Continue and let it finish
		await client.continue_()

		try {
			await client.waitForEvent('terminated', 5000)
		} catch {}

		console.log('  ✓ Variable panel object display test passed!')
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
		const response = await fetch(`http://localhost:3003/health`)
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

		// If testing nsjail mode, verify nsjail is enabled in health response
		if (TEST_NSJAIL) {
			if (data.nsjail !== true) {
				console.log('  ❌ FAIL: nsjail should be enabled but health shows false')
				console.log('  → Make sure you started the server with: bun run dap_debug_service.ts --nsjail')
				return false
			}
			console.log('  ✓ nsjail mode confirmed enabled')
		}

		console.log('  ✓ Health endpoint works!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		return false
	}
}

/**
 * Test Bun/TypeScript autoinstall via unified service.
 * Requires the service to be started with --windmill pointing to the windmill binary.
 */
async function testBunAutoinstall(): Promise<boolean> {
	console.log('\n=== Testing Bun Autoinstall (TypeScript Endpoint) ===')
	console.log('  Note: This test requires the service to be started with:')
	console.log('    bun run dap_debug_service.ts --windmill /path/to/windmill')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()

		// Launch with code that uses lodash (must be auto-installed)
		await client.launch(TS_AUTOINSTALL_CODE, true, { items: [1, 2, 3, 4, 5] })
		console.log('  Launched with lodash import')

		await initP

		// Wait for termination with longer timeout (dependency installation can take time)
		try {
			await client.waitForEvent('terminated', 60000)
		} catch {}

		await new Promise(r => setTimeout(r, 1000))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		const result = client.getResult() as { sum?: number; max?: number } | undefined
		console.log(`  Result: ${JSON.stringify(result)}`)

		// Verify lodash functions worked correctly
		const hasSum = output.some(o => o.includes('Sum:') && o.includes('15'))
		const hasMax = output.some(o => o.includes('Max:') && o.includes('5'))

		if (!hasSum) {
			console.log('  ❌ FAIL: Missing correct sum output (expected 15)')
			return false
		}
		if (!hasMax) {
			console.log('  ❌ FAIL: Missing correct max output (expected 5)')
			return false
		}

		if (!result || result.sum !== 15 || result.max !== 5) {
			console.log('  ❌ FAIL: Incorrect result from lodash functions')
			return false
		}

		console.log('  ✓ Bun autoinstall test passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		console.log('  Make sure the service is started with --windmill flag')
		return false
	} finally {
		client.disconnect()
	}
}

/**
 * Test versioned imports (e.g., "number-to-words@1") via unified service.
 * This syntax is supported by bun_executor in the backend but needs explicit handling in the debugger.
 * Requires the service to be started with --windmill pointing to the windmill binary.
 */
async function testVersionedImports(): Promise<boolean> {
	console.log('\n=== Testing Versioned Imports (TypeScript) ===')
	console.log('  Note: This test requires the service to be started with:')
	console.log('    bun run dap_debug_service.ts --windmill /path/to/windmill')
	const client = new TestClient()

	try {
		await client.connect('/typescript')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.ts', [])
		await client.configurationDone()

		// Launch with code that uses number-to-words@1 syntax (version specifier in import)
		await client.launch(TS_VERSIONED_IMPORT_CODE, true, { num: 42 })
		console.log('  Launched with versioned import (number-to-words@1)')

		await initP

		// Wait for termination with longer timeout (dependency installation can take time)
		try {
			await client.waitForEvent('terminated', 60000)
		} catch {}

		await new Promise(r => setTimeout(r, 1000))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		const result = client.getResult() as { num?: number; words?: string } | undefined
		console.log(`  Result: ${JSON.stringify(result)}`)

		// Verify number-to-words converted 42 correctly
		const hasWordsOutput = output.some(o => o.includes('Number in words:') && o.includes('forty'))

		if (!hasWordsOutput) {
			console.log('  ❌ FAIL: Missing correct "forty" output from number-to-words')
			return false
		}

		if (!result || result.num !== 42) {
			console.log('  ❌ FAIL: Incorrect num in result')
			return false
		}

		if (!result.words || !result.words.toLowerCase().includes('forty')) {
			console.log('  ❌ FAIL: Incorrect words in result (expected "forty-two")')
			return false
		}

		console.log('  ✓ Versioned imports test passed!')
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		console.log('  Make sure the service is started with --windmill flag')
		return false
	} finally {
		client.disconnect()
	}
}

/**
 * Test Python autoinstall via unified service.
 * Requires the service to be started with --windmill pointing to the windmill binary.
 */
async function testPythonAutoinstall(): Promise<boolean> {
	console.log('\n=== Testing Python Autoinstall ===')
	console.log('  Note: This test requires the service to be started with:')
	console.log('    bun run dap_debug_service.ts --windmill /path/to/windmill')
	const client = new TestClient()

	try {
		await client.connect('/python')
		console.log('  Connected')

		await client.initialize()
		console.log('  Initialized')

		const initP = client.waitForEvent('initialized')
		await client.setBreakpoints('/test.py', [])
		await client.configurationDone()

		// Launch with code that uses requests (must be auto-installed)
		await client.launch(PYTHON_AUTOINSTALL_CODE, true, {})
		console.log('  Launched with requests import')

		await initP

		// Wait for termination with longer timeout (dependency installation can take time)
		try {
			await client.waitForEvent('terminated', 120000)
		} catch {}

		await new Promise(r => setTimeout(r, 1000))

		const output = client.getOutput()
		console.log(`  Output: ${JSON.stringify(output)}`)

		const result = client.getResult() as { version?: string; imported?: boolean } | undefined
		console.log(`  Result: ${JSON.stringify(result)}`)

		// Verify requests was imported successfully
		const hasVersionOutput = output.some(o => o.includes('Requests version:'))

		if (!hasVersionOutput) {
			console.log('  ❌ FAIL: Missing requests version output')
			return false
		}

		if (!result || result.imported !== true) {
			console.log('  ❌ FAIL: requests module was not imported successfully')
			return false
		}

		if (!result.version || typeof result.version !== 'string') {
			console.log('  ❌ FAIL: requests version not returned correctly')
			return false
		}

		console.log(`  ✓ Python autoinstall test passed! (requests ${result.version})`)
		return true
	} catch (error) {
		console.log(`  ❌ FAIL: ${error}`)
		console.log('  Make sure the service is started with --windmill flag')
		return false
	} finally {
		client.disconnect()
	}
}

async function checkNsjailAvailable(): Promise<boolean> {
	try {
		const proc = spawn({
			cmd: ['nsjail', '--version'],
			stdout: 'pipe',
			stderr: 'pipe'
		})
		const exitCode = await proc.exited
		return exitCode === 0
	} catch {
		return false
	}
}

async function main() {
	console.log('='.repeat(60))
	console.log('DAP Debug Service Tests')
	if (TEST_NSJAIL) {
		console.log('Mode: NSJAIL ENABLED')
		const nsjailAvailable = await checkNsjailAvailable()
		if (!nsjailAvailable) {
			console.log('\n⚠️  WARNING: nsjail not found in PATH')
			console.log('   Tests will verify nsjail configuration is passed correctly,')
			console.log('   but actual sandboxed execution cannot be tested.')
		}
	} else if (TEST_AUTOINSTALL) {
		console.log('Mode: AUTOINSTALL TEST')
		console.log('Make sure the service is started with: --windmill /path/to/windmill')
	} else {
		console.log('Mode: Standard (no nsjail)')
	}
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

	// Test environment variables (TypeScript)
	if (await testEnvVarsTypescript()) {
		passed++
	} else {
		failed++
	}

	// Test environment variables (Python)
	if (await testEnvVarsPython()) {
		passed++
	} else {
		failed++
	}

	// Test console object output formatting
	if (await testConsoleObjectOutput()) {
		passed++
	} else {
		failed++
	}

	// Test variable panel object display
	if (await testVariableObjectDisplay()) {
		passed++
	} else {
		failed++
	}

	// Autoinstall tests (only run when --test-autoinstall is passed)
	if (TEST_AUTOINSTALL) {
		// Test Bun autoinstall
		if (await testBunAutoinstall()) {
			passed++
		} else {
			failed++
		}

		// Test Python autoinstall
		if (await testPythonAutoinstall()) {
			passed++
		} else {
			failed++
		}

		// Test versioned imports (e.g., "number-to-words@1")
		if (await testVersionedImports()) {
			passed++
		} else {
			failed++
		}
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
