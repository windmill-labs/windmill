#!/usr/bin/env bun
/**
 * Lightweight DAP (Debug Adapter Protocol) WebSocket Server for Bun/TypeScript debugging.
 *
 * This server acts as a bridge between a WebSocket client (Monaco editor) and Bun's
 * built-in debugging capabilities using the WebKit Inspector Protocol (not V8/Chrome DevTools).
 *
 * Key differences from V8/Node.js debugging:
 * - Uses WebKit Inspector Protocol (similar to Safari DevTools)
 * - Requires Inspector.enable, Debugger.setPauseOnDebuggerStatements, and Inspector.initialized
 * - Breakpoints must be activated with Debugger.setBreakpointsActive
 * - Console output comes via Console.messageAdded instead of Runtime.consoleAPICalled
 *
 * It implements a minimal subset of DAP to support basic TypeScript debugging:
 * - Setting breakpoints
 * - Stepping through code (step in, step over, step out, continue)
 * - Inspecting variables and stack frames
 * - Evaluating expressions
 *
 * Usage:
 *    bun run dap_websocket_server_bun.ts [--port PORT] [--host HOST]
 */

import { spawn, type Subprocess } from 'bun'
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

// Types for V8 Inspector Protocol
interface V8Message {
	id?: number
	method?: string
	params?: Record<string, unknown>
	result?: Record<string, unknown>
	error?: { message: string; code?: number }
}

interface V8CallFrame {
	callFrameId: string
	functionName: string
	location: {
		scriptId: string
		lineNumber: number
		columnNumber: number
	}
	scopeChain: Array<{
		type: string
		object: { objectId?: string }
		name?: string
	}>
	this?: { objectId?: string }
}

interface V8Script {
	scriptId: string
	url: string
	startLine: number
	startColumn: number
	endLine: number
	endColumn: number
	hash: string
}

// DAP Message types
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

interface StackFrame {
	id: number
	name: string
	source: { path: string; name: string }
	line: number
	column: number
}

interface Variable {
	name: string
	value: string
	type: string
	variablesReference: number
}

interface Breakpoint {
	id: number
	verified: boolean
	line: number
	source?: { path: string; name?: string }
}

// Logging
const LOG_LEVEL = process.env.LOG_LEVEL || 'DEBUG'
const logger = {
	debug: (...args: unknown[]) => {
		if (LOG_LEVEL === 'DEBUG') console.log('[DEBUG]', new Date().toISOString(), ...args)
	},
	info: (...args: unknown[]) => console.log('[INFO]', new Date().toISOString(), ...args),
	warn: (...args: unknown[]) => console.warn('[WARN]', new Date().toISOString(), ...args),
	error: (...args: unknown[]) => console.error('[ERROR]', new Date().toISOString(), ...args)
}

/**
 * Manages a debug session with a Bun subprocess.
 */
class DebugSession {
	private ws: WebSocket
	private seq = 1
	private initialized = false
	private configured = false
	private running = false

	// Bun subprocess
	private process: Subprocess | null = null
	private inspectorWs: WebSocket | null = null
	private inspectorSeq = 1
	private pendingInspectorRequests = new Map<
		number,
		{ resolve: (value: V8Message) => void; reject: (error: Error) => void }
	>()

	// Script and breakpoint tracking
	private scriptPath: string | null = null
	private tempDir: string | null = null
	private tempFile: string | null = null
	private breakpoints = new Map<string, number[]>() // file -> line numbers
	private breakpointIds = new Map<string, string[]>() // file -> V8 breakpoint IDs

	// Script ID mapping (V8 uses script IDs, we need to map to file paths)
	private scripts = new Map<string, V8Script>()
	private mainScriptId: string | null = null

	// Call frames when paused
	private callFrames: V8CallFrame[] = []
	private variablesRefCounter = 1
	private scopesMap = new Map<number, { type: string; objectId: string; frameIndex: number }>()
	private objectsMap = new Map<number, string>() // variablesRef -> objectId

	// For calling main()
	private callMain = false
	private mainArgs: Record<string, unknown> = {}

	constructor(ws: WebSocket) {
		this.ws = ws
	}

	private nextSeq(): number {
		return this.seq++
	}

	private nextInspectorSeq(): number {
		return this.inspectorSeq++
	}

	private nextVarRef(): number {
		return this.variablesRefCounter++
	}

	/**
	 * Send a DAP message to the client.
	 */
	private sendMessage(msg: DAPMessage): void {
		const data = JSON.stringify(msg)
		logger.debug('Sending DAP:', data)
		this.ws.send(data)
	}

	/**
	 * Send a DAP response.
	 */
	private sendResponse(
		request: DAPMessage,
		success = true,
		body: Record<string, unknown> = {},
		message = ''
	): void {
		this.sendMessage({
			seq: this.nextSeq(),
			type: 'response',
			command: request.command || '',
			request_seq: request.seq,
			success,
			message,
			body
		})
	}

	/**
	 * Send a DAP event.
	 */
	private sendEvent(event: string, body: Record<string, unknown> = {}): void {
		this.sendMessage({
			seq: this.nextSeq(),
			type: 'event',
			event,
			body
		})
	}

	/**
	 * Send a V8 Inspector command and wait for response.
	 */
	private async sendInspectorCommand(
		method: string,
		params: Record<string, unknown> = {}
	): Promise<V8Message> {
		if (!this.inspectorWs || this.inspectorWs.readyState !== WebSocket.OPEN) {
			throw new Error('Inspector not connected')
		}

		const id = this.nextInspectorSeq()
		const message = { id, method, params }

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pendingInspectorRequests.delete(id)
				reject(new Error(`Inspector command timeout: ${method}`))
			}, 10000)

			this.pendingInspectorRequests.set(id, {
				resolve: (value) => {
					clearTimeout(timeout)
					resolve(value)
				},
				reject: (error) => {
					clearTimeout(timeout)
					reject(error)
				}
			})

			logger.debug('Sending to inspector:', JSON.stringify(message))
			this.inspectorWs!.send(JSON.stringify(message))
		})
	}

	/**
	 * Handle messages from the V8 Inspector.
	 */
	private handleInspectorMessage(data: string): void {
		try {
			const message: V8Message = JSON.parse(data)
			logger.debug('Inspector message:', data)

			// Handle responses
			if (message.id !== undefined) {
				const pending = this.pendingInspectorRequests.get(message.id)
				if (pending) {
					this.pendingInspectorRequests.delete(message.id)
					if (message.error) {
						pending.reject(new Error(message.error.message))
					} else {
						pending.resolve(message)
					}
				}
				return
			}

			// Handle events
			if (message.method) {
				this.handleInspectorEvent(message)
			}
		} catch (error) {
			logger.error('Failed to parse inspector message:', error)
		}
	}

	/**
	 * Handle V8 Inspector events.
	 */
	private handleInspectorEvent(message: V8Message): void {
		const params = message.params || {}

		switch (message.method) {
			case 'Debugger.scriptParsed':
				this.handleScriptParsed(params as unknown as V8Script)
				break

			case 'Debugger.paused':
				this.handlePaused(params)
				break

			case 'Debugger.resumed':
				this.sendEvent('continued', { threadId: 1 })
				break

			case 'Runtime.consoleAPICalled':
				this.handleConsoleOutput(params)
				break

			case 'Console.messageAdded':
				// Bun's WebKit Inspector uses Console.messageAdded instead of Runtime.consoleAPICalled
				this.handleConsoleMessageAdded(params)
				break

			case 'Runtime.exceptionThrown':
				this.handleException(params)
				break

			case 'Debugger.breakpointResolved':
				logger.info(`Breakpoint resolved: ${JSON.stringify(params)}`)
				break

			default:
				logger.debug('Unhandled inspector event:', message.method)
		}
	}

	/**
	 * Handle script parsed event - track script IDs.
	 */
	private handleScriptParsed(script: V8Script): void {
		this.scripts.set(script.scriptId, script)

		// Identify the main script
		if (script.url && this.scriptPath && script.url.endsWith(this.scriptPath.split('/').pop()!)) {
			this.mainScriptId = script.scriptId
			logger.info(`Main script parsed: ${script.url} (ID: ${script.scriptId})`)
		}
	}

	/**
	 * Apply breakpoints using URL pattern (works before script is parsed).
	 * This is called before the script starts running.
	 */
	private async applyBreakpointsByUrl(): Promise<void> {
		if (!this.scriptPath) return

		// Clear existing breakpoints first
		for (const [filePath, ids] of this.breakpointIds) {
			for (const id of ids) {
				try {
					await this.sendInspectorCommand('Debugger.removeBreakpoint', { breakpointId: id })
				} catch {
					// Ignore errors during removal
				}
			}
		}
		this.breakpointIds.clear()

		// Set breakpoints by URL pattern
		for (const [filePath, lines] of this.breakpoints) {
			const ids: string[] = []
			for (const line of lines) {
				try {
					// Use the actual script path as URL regex
					// For file URLs, we match the end of the path
					const urlRegex = this.scriptPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

					// User line numbers are 1-indexed
					// We injected a debugger statement at line 0 (1 line)
					// So user line N needs to become line N (0-indexed: N-1, +1 for injected = N)
					// WebKit uses 0-indexed lines, so we use line directly (line is already 1-indexed)
					const adjustedLine = line // User's 1-indexed line = 0-indexed line in temp file

					const response = await this.sendInspectorCommand('Debugger.setBreakpointByUrl', {
						lineNumber: adjustedLine,
						urlRegex,
						columnNumber: 0
					})
					if (response.result?.breakpointId) {
						ids.push(response.result.breakpointId as string)
						logger.info(`Breakpoint set by URL at user line ${line} (temp file line ${adjustedLine + 1}, ID: ${response.result.breakpointId})`)
					}
				} catch (error) {
					logger.error(`Failed to set breakpoint at line ${line}:`, error)
				}
			}
			this.breakpointIds.set(filePath, ids)
		}
	}

	// Track if this is the initial pause from our injected debugger statement
	private initialPauseDone = false

	/**
	 * Handle paused event.
	 */
	private async handlePaused(params: Record<string, unknown>): Promise<void> {
		const callFrames = params.callFrames as V8CallFrame[] | undefined
		const reason = params.reason as string

		if (callFrames) {
			this.callFrames = callFrames
		}

		// Check if this is the initial pause from our injected debugger statement
		// We detect this by checking if we're on line 0 (our injected line) and reason is DebuggerStatement
		const isInitialPause =
			!this.initialPauseDone &&
			reason === 'DebuggerStatement' &&
			callFrames &&
			callFrames.length > 0 &&
			callFrames[0].location.lineNumber === 0

		if (isInitialPause) {
			this.initialPauseDone = true
			logger.info('Initial pause from injected debugger statement - auto-resuming')

			// Auto-resume to continue to actual user code
			try {
				await this.sendInspectorCommand('Debugger.resume', {})
				logger.info('Auto-resumed after initial pause')
			} catch (error) {
				logger.error('Failed to auto-resume:', error)
			}
			return // Don't send stopped event for the initial debugger pause
		}

		// Mark initial pause as done if we get here
		this.initialPauseDone = true

		// Map reason to DAP reason
		let dapReason: string
		switch (reason) {
			case 'Breakpoint':
			case 'breakpoint':
				dapReason = 'breakpoint'
				break
			case 'DebuggerStatement':
				dapReason = 'breakpoint' // Treat user's debugger; statements as breakpoints
				break
			case 'step':
				dapReason = 'step'
				break
			case 'exception':
				dapReason = 'exception'
				break
			case 'debugCommand':
				dapReason = 'pause'
				break
			default:
				dapReason = 'step'
		}

		// Get current line from first call frame
		// lineNumber is 0-indexed in WebKit inspector
		// We injected 1 line at the start, so:
		// - temp file line 0 = our debugger statement
		// - temp file line 1 = user line 1
		// - temp file line N = user line N (because inspector 0-indexed + 1 offset cancel out)
		let line: number | undefined
		if (callFrames && callFrames.length > 0) {
			// Convert 0-indexed inspector line to 1-indexed user line, accounting for injected line
			const inspectorLine = callFrames[0].location.lineNumber
			line = inspectorLine // inspectorLine 0-indexed + 1 for display - 1 for injected = same value
		}

		this.sendEvent('stopped', {
			reason: dapReason,
			threadId: 1,
			allThreadsStopped: true,
			line
		})
	}

	/**
	 * Handle console output (V8/Chrome format).
	 */
	private handleConsoleOutput(params: Record<string, unknown>): void {
		const type = params.type as string
		const args = params.args as Array<{ type: string; value?: unknown; description?: string }>

		if (!args) return

		const output = args
			.map((arg) => {
				if (arg.value !== undefined) return String(arg.value)
				if (arg.description) return arg.description
				return ''
			})
			.join(' ')

		this.sendEvent('output', {
			category: type === 'error' ? 'stderr' : 'stdout',
			output: output + '\n'
		})
	}

	/**
	 * Handle console output (WebKit/Bun format - Console.messageAdded).
	 */
	private handleConsoleMessageAdded(params: Record<string, unknown>): void {
		const message = params.message as {
			source?: string
			level?: string
			text?: string
			type?: string
			parameters?: Array<{ type: string; value?: unknown; description?: string }>
		}

		if (!message) return

		let output: string
		if (message.parameters && message.parameters.length > 0) {
			output = message.parameters
				.map((p) => {
					if (p.value !== undefined) return String(p.value)
					if (p.description) return p.description
					return ''
				})
				.join(' ')
		} else {
			output = message.text || ''
		}

		const category = message.level === 'error' || message.level === 'warning' ? 'stderr' : 'stdout'
		this.sendEvent('output', {
			category,
			output: output + '\n'
		})
	}

	/**
	 * Handle runtime exceptions.
	 */
	private handleException(params: Record<string, unknown>): void {
		const exceptionDetails = params.exceptionDetails as {
			text?: string
			exception?: { description?: string }
		}

		if (exceptionDetails) {
			const message =
				exceptionDetails.exception?.description || exceptionDetails.text || 'Unknown exception'
			this.sendEvent('output', {
				category: 'stderr',
				output: `Exception: ${message}\n`
			})
		}
	}

	/**
	 * Handle the 'initialize' request.
	 */
	async handleInitialize(request: DAPMessage): Promise<void> {
		const capabilities = {
			supportsConfigurationDoneRequest: true,
			supportsFunctionBreakpoints: false,
			supportsConditionalBreakpoints: false,
			supportsHitConditionalBreakpoints: false,
			supportsEvaluateForHovers: true,
			exceptionBreakpointFilters: [],
			supportsStepBack: false,
			supportsSetVariable: false,
			supportsRestartFrame: false,
			supportsGotoTargetsRequest: false,
			supportsStepInTargetsRequest: false,
			supportsCompletionsRequest: false,
			supportsModulesRequest: false,
			supportsExceptionOptions: false,
			supportsValueFormattingOptions: false,
			supportsExceptionInfoRequest: false,
			supportTerminateDebuggee: true,
			supportsDelayedStackTraceLoading: false,
			supportsLoadedSourcesRequest: false,
			supportsLogPoints: false,
			supportsTerminateThreadsRequest: false,
			supportsSetExpression: false,
			supportsTerminateRequest: true,
			supportsDataBreakpoints: false,
			supportsReadMemoryRequest: false,
			supportsDisassembleRequest: false,
			supportsCancelRequest: false,
			supportsBreakpointLocationsRequest: false
		}

		this.sendResponse(request, true, capabilities)
		this.initialized = true
		this.sendEvent('initialized')
	}

	/**
	 * Handle the 'setBreakpoints' request.
	 */
	async handleSetBreakpoints(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const source = args.source as { path?: string } | undefined
		const sourcePath = source?.path || ''
		const breakpointsData = (args.breakpoints as Array<{ line: number }>) || []

		const verifiedBreakpoints: Breakpoint[] = []
		const lineNumbers: number[] = []

		for (const bp of breakpointsData) {
			const line = bp.line
			lineNumbers.push(line)
			verifiedBreakpoints.push({
				id: verifiedBreakpoints.length + 1,
				verified: true,
				line,
				source: { path: sourcePath }
			})
		}

		// Store breakpoints
		this.breakpoints.set(sourcePath, lineNumbers)
		logger.info(`Stored breakpoints at lines ${lineNumbers} for ${sourcePath}`)

		// If inspector is connected, apply breakpoints immediately
		if (this.inspectorWs) {
			await this.applyBreakpointsByUrl()
		}

		this.sendResponse(request, true, { breakpoints: verifiedBreakpoints })
	}

	/**
	 * Handle the 'configurationDone' request.
	 */
	async handleConfigurationDone(request: DAPMessage): Promise<void> {
		this.configured = true
		this.sendResponse(request)
	}

	/**
	 * Handle the 'launch' request.
	 */
	async handleLaunch(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		let code = args.code as string | undefined
		this.scriptPath = args.program as string | undefined
		const cwd = (args.cwd as string) || process.cwd()
		this.callMain = (args.callMain as boolean) || false
		this.mainArgs = (args.args as Record<string, unknown>) || {}

		if (!this.scriptPath && !code) {
			this.sendResponse(request, false, {}, 'No program or code specified')
			return
		}

		// Reset state for new launch
		this.initialPauseDone = false

		// Inject a debugger statement at the beginning to force initial pause
		// This gives us a known pause point where we can ensure breakpoints are set
		code = `debugger; // Auto-injected by Windmill debugger\n${code || ''}`

		// If callMain is true, append a call to main() with the provided args
		if (this.callMain && code) {
			const argsStr = Object.entries(this.mainArgs)
				.map(([k, v]) => `${k}: ${JSON.stringify(v)}`)
				.join(', ')
			code =
				code +
				`\n\n// Auto-generated call to main entrypoint\nconst __windmill_result__ = await main({${argsStr}});\nconsole.log("__WINDMILL_RESULT__:" + JSON.stringify(__windmill_result__));\n`
			logger.info(`Added main() call with args: ${argsStr}`)
		}

		// Write code to temp file if provided
		if (code && !this.scriptPath) {
			try {
				this.tempDir = await mkdtemp(join(tmpdir(), 'windmill_debug_'))
				this.tempFile = join(this.tempDir, 'script.ts')
				await writeFile(this.tempFile, code)
				this.scriptPath = this.tempFile
				logger.info(`Wrote code to ${this.tempFile}`)
			} catch (error) {
				this.sendResponse(request, false, {}, `Failed to create temp file: ${error}`)
				return
			}
		}

		this.sendResponse(request)

		// Start Bun with inspect-wait
		try {
			await this.startBunProcess(cwd)
		} catch (error) {
			this.sendEvent('output', { category: 'stderr', output: `Failed to start Bun: ${error}\n` })
			this.sendEvent('terminated', { error: String(error) })
		}
	}

	// Store the inspector WebSocket URL when parsed from stderr
	private inspectorWsUrl: string | null = null
	private inspectorWsUrlPromise: { resolve: (url: string) => void; reject: (error: Error) => void } | null = null

	/**
	 * Start the Bun subprocess with debugging enabled.
	 */
	private async startBunProcess(cwd: string): Promise<void> {
		if (!this.scriptPath) {
			throw new Error('No script path')
		}

		const inspectPort = 9229 + Math.floor(Math.random() * 1000)
		const inspectUrl = `127.0.0.1:${inspectPort}`

		// Use --inspect-wait to wait for debugger connection before running
		logger.info(`Starting Bun with --inspect-wait=${inspectUrl}`)

		// Create a promise to wait for the WebSocket URL
		const wsUrlPromise = new Promise<string>((resolve, reject) => {
			this.inspectorWsUrlPromise = { resolve, reject }
			// Timeout after 10 seconds
			setTimeout(() => {
				if (this.inspectorWsUrlPromise) {
					reject(new Error('Timeout waiting for inspector WebSocket URL'))
					this.inspectorWsUrlPromise = null
				}
			}, 10000)
		})

		this.process = spawn({
			cmd: ['bun', `--inspect-wait=${inspectUrl}`, this.scriptPath],
			cwd,
			stdout: 'pipe',
			stderr: 'pipe',
			env: {
				...process.env,
				NODE_ENV: 'development'
			}
		})

		// Note: We don't read stdout directly because Console.messageAdded events
		// from the WebKit inspector handle console.log output. Reading stdout
		// would cause duplicate output.

		// Handle stderr (look for inspector URL and forward non-inspector output)
		this.readStderrForInspectorUrl(this.process.stderr)

		// Wait for the WebSocket URL to be parsed from stderr
		const wsUrl = await wsUrlPromise
		logger.info(`Got inspector WebSocket URL: ${wsUrl}`)

		// Connect to the inspector
		await this.connectToInspector(wsUrl)

		// Wait for process to exit
		this.process.exited.then(async () => {
			logger.info('Bun process exited')
			this.running = false
			await this.cleanup()
			this.sendEvent('terminated')
		})
	}

	/**
	 * Read from a readable stream and send output events.
	 */
	private async readStream(
		stream: ReadableStream<Uint8Array> | null,
		category: 'stdout' | 'stderr'
	): Promise<void> {
		if (!stream) return

		const reader = stream.getReader()
		const decoder = new TextDecoder()

		try {
			while (true) {
				const { done, value } = await reader.read()
				if (done) break

				const text = decoder.decode(value)
				this.sendEvent('output', { category, output: text })
			}
		} catch (error) {
			logger.debug(`Stream ${category} ended:`, error)
		}
	}

	/**
	 * Read stderr specifically to extract the inspector WebSocket URL.
	 */
	private async readStderrForInspectorUrl(stream: ReadableStream<Uint8Array> | null): Promise<void> {
		if (!stream) return

		const reader = stream.getReader()
		const decoder = new TextDecoder()
		let buffer = ''

		try {
			while (true) {
				const { done, value } = await reader.read()
				if (done) break

				const text = decoder.decode(value)
				buffer += text

				// Look for the WebSocket URL in Bun's inspector output
				// Format: "ws://127.0.0.1:9229/xxxxx"
				const wsMatch = buffer.match(/ws:\/\/[\d.]+:\d+\/[a-z0-9]+/i)
				if (wsMatch && this.inspectorWsUrlPromise) {
					const wsUrl = wsMatch[0]
					logger.info(`Found inspector WebSocket URL in stderr: ${wsUrl}`)
					this.inspectorWsUrl = wsUrl
					this.inspectorWsUrlPromise.resolve(wsUrl)
					this.inspectorWsUrlPromise = null
				}

				// Forward non-inspector output to the client
				// Skip the Bun Inspector banner
				if (!text.includes('Bun Inspector') && !text.includes('Listening:') &&
					!text.includes('ws://') && !text.includes('debug.bun.sh')) {
					this.sendEvent('output', { category: 'stderr', output: text })
				}
			}
		} catch (error) {
			logger.debug('Stderr stream ended:', error)
		}
	}

	/**
	 * Connect to Bun's inspector WebSocket.
	 */
	private async connectToInspector(wsUrl: string): Promise<void> {
		logger.info(`Connecting to inspector at ${wsUrl}`)

		return new Promise((resolve, reject) => {
			this.inspectorWs = new WebSocket(wsUrl)

			const timeout = setTimeout(() => {
				reject(new Error('Inspector connection timeout'))
			}, 5000)

			this.inspectorWs.onopen = async () => {
				clearTimeout(timeout)
				logger.info('Connected to inspector')

				try {
					// Enable inspector domain first (required for Bun's WebKit Inspector Protocol)
					await this.sendInspectorCommand('Inspector.enable', {})

					// Enable console for output
					await this.sendInspectorCommand('Console.enable', {})

					// Enable the debugger domain
					await this.sendInspectorCommand('Debugger.enable', {})

					// Enable runtime domain for evaluation
					await this.sendInspectorCommand('Runtime.enable', {})

					// CRITICAL: Activate breakpoints (required for Bun)
					await this.sendInspectorCommand('Debugger.setBreakpointsActive', { active: true })

					// CRITICAL: Enable pause on debugger statements (required for Bun)
					await this.sendInspectorCommand('Debugger.setPauseOnDebuggerStatements', { enabled: true })

					// Set pause on exceptions
					await this.sendInspectorCommand('Debugger.setPauseOnExceptions', {
						state: 'uncaught'
					})

					// Set breakpoints BEFORE starting execution
					await this.applyBreakpointsByUrl()

					this.running = true

					// CRITICAL: Call Inspector.initialized to start script execution
					// Without this, Bun waits indefinitely with --inspect-wait
					logger.info('Starting script execution with Inspector.initialized...')
					await this.sendInspectorCommand('Inspector.initialized', {})

					resolve()
				} catch (error) {
					reject(error)
				}
			}

			this.inspectorWs.onmessage = (event) => {
				this.handleInspectorMessage(event.data as string)
			}

			this.inspectorWs.onerror = (error) => {
				logger.error('Inspector WebSocket error:', error)
			}

			this.inspectorWs.onclose = () => {
				logger.info('Inspector WebSocket closed')
				this.inspectorWs = null
			}
		})
	}

	/**
	 * Handle the 'threads' request.
	 */
	async handleThreads(request: DAPMessage): Promise<void> {
		this.sendResponse(request, true, {
			threads: [{ id: 1, name: 'MainThread' }]
		})
	}

	/**
	 * Handle the 'stackTrace' request.
	 */
	async handleStackTrace(request: DAPMessage): Promise<void> {
		const stackFrames: StackFrame[] = []

		for (let i = 0; i < this.callFrames.length; i++) {
			const frame = this.callFrames[i]
			const script = this.scripts.get(frame.location.scriptId)

			// Stop at module level
			if (frame.functionName === '' && i > 0) {
				// Skip anonymous functions after the first frame
				continue
			}

			stackFrames.push({
				id: i + 1,
				name: frame.functionName || '<anonymous>',
				source: {
					path: script?.url || this.scriptPath || '<unknown>',
					name: script?.url?.split('/').pop() || 'script.ts'
				},
				line: frame.location.lineNumber + 1, // Convert to 1-indexed
				column: frame.location.columnNumber + 1
			})
		}

		this.sendResponse(request, true, {
			stackFrames,
			totalFrames: stackFrames.length
		})
	}

	/**
	 * Handle the 'scopes' request.
	 */
	async handleScopes(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const frameId = (args.frameId as number) || 1
		const frameIndex = frameId - 1

		const frame = this.callFrames[frameIndex]
		if (!frame) {
			this.sendResponse(request, true, { scopes: [] })
			return
		}

		const scopes: Array<{ name: string; variablesReference: number; expensive: boolean }> = []

		for (const scope of frame.scopeChain) {
			if (scope.type === 'global') continue // Skip global scope for performance

			const ref = this.nextVarRef()
			const objectId = scope.object.objectId

			if (objectId) {
				this.scopesMap.set(ref, { type: scope.type, objectId, frameIndex })

				scopes.push({
					name: scope.type.charAt(0).toUpperCase() + scope.type.slice(1),
					variablesReference: ref,
					expensive: scope.type === 'global'
				})
			}
		}

		this.sendResponse(request, true, { scopes })
	}

	/**
	 * Handle the 'variables' request.
	 */
	async handleVariables(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const variablesRef = (args.variablesReference as number) || 0

		const scopeInfo = this.scopesMap.get(variablesRef)
		const objectId = this.objectsMap.get(variablesRef) || scopeInfo?.objectId

		if (!objectId) {
			this.sendResponse(request, true, { variables: [] })
			return
		}

		try {
			const response = await this.sendInspectorCommand('Runtime.getProperties', {
				objectId,
				ownProperties: true,
				generatePreview: true
			})

			const variables: Variable[] = []
			const properties = (response.result?.result as Array<{
				name: string
				value?: {
					type: string
					value?: unknown
					description?: string
					objectId?: string
					subtype?: string
				}
			}>) || []

			for (const prop of properties) {
				if (!prop.value) continue

				let value: string
				let varRef = 0

				if (prop.value.type === 'object' && prop.value.objectId) {
					// Create a reference for nested objects
					varRef = this.nextVarRef()
					this.objectsMap.set(varRef, prop.value.objectId)
					value = prop.value.description || `[${prop.value.subtype || 'Object'}]`
				} else if (prop.value.value !== undefined) {
					value = JSON.stringify(prop.value.value)
				} else {
					value = prop.value.description || String(prop.value.type)
				}

				variables.push({
					name: prop.name,
					value,
					type: prop.value.type,
					variablesReference: varRef
				})
			}

			this.sendResponse(request, true, { variables })
		} catch (error) {
			logger.error('Failed to get variables:', error)
			this.sendResponse(request, true, { variables: [] })
		}
	}

	/**
	 * Handle the 'evaluate' request.
	 */
	async handleEvaluate(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const expression = (args.expression as string) || ''
		const frameId = args.frameId as number | undefined

		try {
			let result: string
			let varRef = 0

			if (frameId !== undefined && this.callFrames[frameId - 1]) {
				const frame = this.callFrames[frameId - 1]
				const response = await this.sendInspectorCommand('Debugger.evaluateOnCallFrame', {
					callFrameId: frame.callFrameId,
					expression,
					returnByValue: true
				})

				const evalResult = response.result?.result as {
					type: string
					value?: unknown
					description?: string
					objectId?: string
				}

				if (evalResult) {
					if (evalResult.value !== undefined) {
						result = JSON.stringify(evalResult.value)
					} else if (evalResult.objectId) {
						varRef = this.nextVarRef()
						this.objectsMap.set(varRef, evalResult.objectId)
						result = evalResult.description || '[Object]'
					} else {
						result = evalResult.description || String(evalResult.type)
					}
				} else {
					result = 'undefined'
				}
			} else {
				const response = await this.sendInspectorCommand('Runtime.evaluate', {
					expression,
					returnByValue: true
				})

				const evalResult = response.result?.result as {
					type: string
					value?: unknown
					description?: string
				}
				result =
					evalResult?.value !== undefined
						? JSON.stringify(evalResult.value)
						: evalResult?.description || 'undefined'
			}

			this.sendResponse(request, true, { result, variablesReference: varRef })
		} catch (error) {
			this.sendResponse(request, true, {
				result: `Error: ${error}`,
				variablesReference: 0
			})
		}
	}

	/**
	 * Handle the 'continue' request.
	 */
	async handleContinue(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.resume', {})
			this.sendResponse(request, true, { allThreadsContinued: true })
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'next' (step over) request.
	 */
	async handleNext(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.stepOver', {})
			this.sendResponse(request)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'stepIn' request.
	 */
	async handleStepIn(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.stepInto', {})
			this.sendResponse(request)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'stepOut' request.
	 */
	async handleStepOut(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.stepOut', {})
			this.sendResponse(request)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'pause' request.
	 */
	async handlePause(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.pause', {})
			this.sendResponse(request)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'disconnect' request.
	 */
	async handleDisconnect(request: DAPMessage): Promise<void> {
		this.running = false
		await this.cleanup()
		this.sendResponse(request)
	}

	/**
	 * Handle the 'terminate' request.
	 */
	async handleTerminate(request: DAPMessage): Promise<void> {
		this.running = false
		await this.cleanup()
		this.sendResponse(request)
		this.sendEvent('terminated')
	}

	/**
	 * Clean up resources.
	 */
	private async cleanup(): Promise<void> {
		// Close inspector connection
		if (this.inspectorWs) {
			this.inspectorWs.close()
			this.inspectorWs = null
		}

		// Kill process
		if (this.process) {
			this.process.kill()
			this.process = null
		}

		// Clean up temp file
		if (this.tempFile) {
			try {
				await unlink(this.tempFile)
			} catch {
				// Ignore
			}
			this.tempFile = null
		}

		if (this.tempDir) {
			try {
				await rmdir(this.tempDir)
			} catch {
				// Ignore
			}
			this.tempDir = null
		}
	}

	/**
	 * Handle an incoming DAP request.
	 */
	async handleRequest(request: DAPMessage): Promise<void> {
		const command = request.command || ''
		logger.debug(`Handling command: ${command}`)

		const handlers: Record<string, (req: DAPMessage) => Promise<void>> = {
			initialize: (req) => this.handleInitialize(req),
			setBreakpoints: (req) => this.handleSetBreakpoints(req),
			configurationDone: (req) => this.handleConfigurationDone(req),
			launch: (req) => this.handleLaunch(req),
			threads: (req) => this.handleThreads(req),
			stackTrace: (req) => this.handleStackTrace(req),
			scopes: (req) => this.handleScopes(req),
			variables: (req) => this.handleVariables(req),
			evaluate: (req) => this.handleEvaluate(req),
			continue: (req) => this.handleContinue(req),
			next: (req) => this.handleNext(req),
			stepIn: (req) => this.handleStepIn(req),
			stepOut: (req) => this.handleStepOut(req),
			pause: (req) => this.handlePause(req),
			disconnect: (req) => this.handleDisconnect(req),
			terminate: (req) => this.handleTerminate(req)
		}

		const handler = handlers[command]
		if (handler) {
			await handler(request)
		} else {
			logger.warn(`Unhandled command: ${command}`)
			this.sendResponse(request, false, {}, `Unsupported command: ${command}`)
		}
	}
}

// Store active sessions by WebSocket
const sessions = new Map<unknown, DebugSession>()

// Parse command line arguments
const args = process.argv.slice(2)
let host = 'localhost'
let port = 5680 // Different port from Python server

for (let i = 0; i < args.length; i++) {
	if (args[i] === '--host' && args[i + 1]) {
		host = args[i + 1]
		i++
	} else if (args[i] === '--port' && args[i + 1]) {
		port = parseInt(args[i + 1], 10)
		i++
	}
}

// Start the server
logger.info(`Starting DAP WebSocket server on ws://${host}:${port}`)

const server = Bun.serve({
	hostname: host,
	port,
	fetch(req, server) {
		// Upgrade to WebSocket
		if (server.upgrade(req)) {
			return undefined as unknown as Response
		}
		return new Response('DAP WebSocket Server for Bun/TypeScript', { status: 200 })
	},
	websocket: {
		open(ws) {
			logger.info('New client connected')
			// Create a wrapper that implements the WebSocket interface expected by DebugSession
			const wsWrapper: WebSocket = {
				send: (data: string) => ws.send(data),
				close: () => ws.close(),
				readyState: WebSocket.OPEN,
				CONNECTING: WebSocket.CONNECTING,
				OPEN: WebSocket.OPEN,
				CLOSING: WebSocket.CLOSING,
				CLOSED: WebSocket.CLOSED
			} as unknown as WebSocket

			const session = new DebugSession(wsWrapper)
			sessions.set(ws, session)
		},
		async message(ws, message) {
			const session = sessions.get(ws)
			if (!session) return

			try {
				const data = JSON.parse(message as string) as DAPMessage
				logger.debug('Received:', JSON.stringify(data))

				if (data.type === 'request') {
					await session.handleRequest(data)
				}
			} catch (error) {
				logger.error('Error handling message:', error)
			}
		},
		close(ws) {
			logger.info('Client disconnected')
			sessions.delete(ws)
		}
	}
})

logger.info(`Server started on ${server.url}`)
