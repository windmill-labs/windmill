#!/usr/bin/env bun
/**
 * Unified DAP Debug Service
 *
 * A containerizable WebSocket service that routes debug requests to either
 * Python (debugpy) or TypeScript/Bun debuggers based on the endpoint hit.
 *
 * Endpoints:
 *   /python     - Python debugging via dap_websocket_server.py (bdb-based)
 *   /typescript - TypeScript/Bun debugging via WebKit Inspector
 *   /bun        - Alias for /typescript
 *
 * Designed to be compatible with nsjail wrapping for sandboxed execution.
 *
 * Usage:
 *   bun run dap_debug_service.ts [options]
 *
 * Options:
 *   --port PORT           Server port (default: 5679)
 *   --host HOST           Server host (default: 0.0.0.0)
 *   --nsjail              Enable nsjail wrapping
 *   --nsjail-config PATH  Path to nsjail config file
 *
 * Environment Variables:
 *   DAP_PORT              Server port
 *   DAP_HOST              Server host
 *   DAP_NSJAIL_ENABLED    Enable nsjail (true/false)
 *   DAP_NSJAIL_CONFIG     Path to nsjail config file
 *   DAP_NSJAIL_PATH       Path to nsjail binary (default: nsjail)
 */

import { spawn, type Subprocess } from 'bun'
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

// Import the working Bun debug session from the standalone server
import { DebugSession as BunDebugSessionWorking } from './dap_websocket_server_bun'

// ============================================================================
// Configuration
// ============================================================================

interface ServiceConfig {
	port: number
	host: string
	nsjail: {
		enabled: boolean
		configPath?: string
		binaryPath: string
		// Additional nsjail options that can be passed
		extraArgs: string[]
	}
	// Paths to debugger binaries (can be overridden for containerized deployments)
	pythonPath: string
	bunPath: string
}

function parseConfig(): ServiceConfig {
	const args = process.argv.slice(2)
	const config: ServiceConfig = {
		port: parseInt(process.env.DAP_PORT || '5679', 10),
		host: process.env.DAP_HOST || '0.0.0.0',
		nsjail: {
			enabled: process.env.DAP_NSJAIL_ENABLED === 'true',
			configPath: process.env.DAP_NSJAIL_CONFIG,
			binaryPath: process.env.DAP_NSJAIL_PATH || 'nsjail',
			extraArgs: []
		},
		pythonPath: process.env.DAP_PYTHON_PATH || 'python3',
		bunPath: process.env.DAP_BUN_PATH || 'bun'
	}

	for (let i = 0; i < args.length; i++) {
		switch (args[i]) {
			case '--port':
				config.port = parseInt(args[++i], 10)
				break
			case '--host':
				config.host = args[++i]
				break
			case '--nsjail':
				config.nsjail.enabled = true
				break
			case '--nsjail-config':
				config.nsjail.configPath = args[++i]
				break
			case '--nsjail-path':
				config.nsjail.binaryPath = args[++i]
				break
			case '--python-path':
				config.pythonPath = args[++i]
				break
			case '--bun-path':
				config.bunPath = args[++i]
				break
		}
	}

	return config
}

const config = parseConfig()

// ============================================================================
// Logging
// ============================================================================

const LOG_LEVEL = process.env.LOG_LEVEL || 'INFO'
const logger = {
	debug: (...args: unknown[]) => {
		if (LOG_LEVEL === 'DEBUG') console.log('[DEBUG]', new Date().toISOString(), ...args)
	},
	info: (...args: unknown[]) => console.log('[INFO]', new Date().toISOString(), ...args),
	warn: (...args: unknown[]) => console.warn('[WARN]', new Date().toISOString(), ...args),
	error: (...args: unknown[]) => console.error('[ERROR]', new Date().toISOString(), ...args)
}

// ============================================================================
// Process Spawning with nsjail Support
// ============================================================================

interface SpawnOptions {
	cmd: string[]
	cwd?: string
	env?: Record<string, string>
	stdout?: 'pipe' | 'inherit'
	stderr?: 'pipe' | 'inherit'
}

/**
 * Spawn a process, optionally wrapped with nsjail.
 * This is the key function for sandboxed execution.
 */
function spawnProcess(options: SpawnOptions): Subprocess {
	let cmd = options.cmd

	if (config.nsjail.enabled) {
		// Build nsjail command
		const nsjailCmd = [config.nsjail.binaryPath]

		// Add config file if specified
		if (config.nsjail.configPath) {
			nsjailCmd.push('--config', config.nsjail.configPath)
		}

		// Add any extra nsjail arguments
		nsjailCmd.push(...config.nsjail.extraArgs)

		// Add working directory binding if specified
		if (options.cwd) {
			nsjailCmd.push('--cwd', options.cwd)
		}

		// Separator and actual command
		nsjailCmd.push('--')
		nsjailCmd.push(...cmd)

		cmd = nsjailCmd
		logger.info(`Spawning with nsjail: ${cmd.join(' ')}`)
	} else {
		logger.info(`Spawning: ${cmd.join(' ')}`)
	}

	return spawn({
		cmd,
		cwd: options.cwd || process.cwd(),
		stdout: options.stdout || 'pipe',
		stderr: options.stderr || 'pipe',
		env: {
			...process.env,
			...options.env
		}
	})
}

// ============================================================================
// DAP Types
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

// ============================================================================
// Base Debug Session
// ============================================================================

abstract class BaseDebugSession {
	protected ws: { send: (data: string) => void; close: () => void }
	protected seq = 1
	protected initialized = false
	protected configured = false
	protected running = false
	protected terminatedSent = false

	protected process: Subprocess | null = null
	protected scriptPath: string | null = null
	protected tempDir: string | null = null
	protected tempFile: string | null = null
	protected breakpoints = new Map<string, number[]>()
	protected callMain = false
	protected mainArgs: Record<string, unknown> = {}
	protected scriptResult: unknown = undefined

	constructor(ws: { send: (data: string) => void; close: () => void }) {
		this.ws = ws
	}

	protected nextSeq(): number {
		return this.seq++
	}

	protected sendMessage(msg: DAPMessage): void {
		const data = JSON.stringify(msg)
		logger.debug('Sending DAP:', data)
		this.ws.send(data)
	}

	protected sendResponse(
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

	protected sendEvent(event: string, body: Record<string, unknown> = {}): void {
		this.sendMessage({
			seq: this.nextSeq(),
			type: 'event',
			event,
			body
		})
	}

	abstract handleRequest(request: DAPMessage): Promise<void>
	abstract cleanup(): Promise<void>
}

// ============================================================================
// TypeScript/Bun Debug Session
// ============================================================================

class BunDebugSession extends BaseDebugSession {
	private inspectorWs: WebSocket | null = null
	private inspectorSeq = 1
	private pendingInspectorRequests = new Map<
		number,
		{ resolve: (value: V8Message) => void; reject: (error: Error) => void }
	>()

	private scripts = new Map<string, V8Script>()
	private mainScriptId: string | null = null
	private mainScriptUrl: string | null = null
	private mainScriptEndLine: number = 0
	private callFrames: V8CallFrame[] = []
	private variablesRefCounter = 1
	private scopesMap = new Map<number, { type: string; objectId: string; frameIndex: number }>()
	private objectsMap = new Map<number, string>()
	private breakpointIds = new Map<string, string[]>()
	private initialPauseDone = false
	private isStepping = false
	private pendingConsoleOutput: Array<Record<string, unknown>> = []
	private pausedAtBreakpoint = false
	private inspectorWsUrl: string | null = null
	private inspectorWsUrlPromise: { resolve: (url: string) => void; reject: (error: Error) => void } | null = null
	private breakpointsSetResolve: (() => void) | null = null
	private breakpointsSet = false

	private nextInspectorSeq(): number {
		return this.inspectorSeq++
	}

	private nextVarRef(): number {
		return this.variablesRefCounter++
	}

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

	private handleInspectorMessage(data: string): void {
		try {
			const message: V8Message = JSON.parse(data)
			logger.debug('Inspector message:', data.substring(0, 200))

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

			if (message.method) {
				this.handleInspectorEvent(message)
			}
		} catch (error) {
			logger.error('Failed to parse inspector message:', error)
		}
	}

	private handleInspectorEvent(message: V8Message): void {
		const params = message.params || {}

		switch (message.method) {
			case 'Debugger.scriptParsed':
				// Handle async - don't await, but let it run
				this.handleScriptParsed(params as unknown as V8Script).catch(err => {
					logger.error('Error in handleScriptParsed:', err)
				})
				break
			case 'Debugger.paused':
				this.handlePaused(params)
				break
			case 'Debugger.resumed':
				this.sendEvent('continued', { threadId: 1 })
				break
			case 'Console.messageAdded':
				this.handleConsoleMessageAdded(params)
				break
			case 'Runtime.exceptionThrown':
				this.handleException(params)
				break
		}
	}

	private async handleScriptParsed(script: V8Script): Promise<void> {
		this.scripts.set(script.scriptId, script)
		const scriptFileName = this.scriptPath?.split('/').pop()
		logger.info(`Script parsed: id=${script.scriptId}, url=${script.url}, lines=${script.startLine}-${script.endLine}`)

		if (script.url && scriptFileName && script.url.endsWith(scriptFileName)) {
			this.mainScriptId = script.scriptId
			this.mainScriptUrl = script.url
			this.mainScriptEndLine = script.endLine
			logger.info(`  *** MATCHED - This is our main script! ***`)
			// NOTE: We don't try to set V8 breakpoints here - they don't work for function bodies.
			// Instead, we rely on injected `debugger;` statements in the code.
		}
	}

	private async waitForBreakpointsSet(): Promise<void> {
		if (this.breakpointsSet) return
		// If no breakpoints to set, don't wait
		if (this.breakpoints.size === 0) {
			this.breakpointsSet = true
			return
		}
		// Wait for breakpoints to be set (with timeout)
		return new Promise((resolve) => {
			if (this.breakpointsSet) {
				resolve()
				return
			}
			this.breakpointsSetResolve = resolve
			// Timeout after 2 seconds - if script wasn't detected, try fallback
			setTimeout(async () => {
				if (this.breakpointsSetResolve) {
					logger.warn('Timeout waiting for breakpoints - script may not have been detected')
					// Try to set breakpoints using scriptPath as fallback
					if (!this.mainScriptUrl && this.scriptPath) {
						logger.info('Attempting fallback breakpoint setting using scriptPath')
						await this.applyBreakpointsWithUrl(this.scriptPath)
					}
					this.breakpointsSetResolve = null
					resolve()
				}
			}, 2000)
		})
	}

	/**
	 * Apply breakpoints using URL pattern (works before script is parsed).
	 * This uses the same logic as dap_websocket_server_bun.ts which is known to work.
	 */
	private async applyBreakpointsByUrl(): Promise<void> {
		logger.info(`applyBreakpointsByUrl called. scriptPath: ${this.scriptPath}, breakpoints: ${JSON.stringify([...this.breakpoints.entries()])}`)
		if (!this.scriptPath) {
			logger.warn('No scriptPath set, skipping breakpoints')
			return
		}

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
			logger.info(`Setting breakpoints for ${filePath}: lines ${lines}`)
			const ids: string[] = []
			for (const line of lines) {
				try {
					// Use the actual script path as URL regex
					const urlRegex = this.scriptPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

					// Bun's WebKit Inspector has a quirk: breakpoints pause AFTER the line executes,
					// not BEFORE like traditional debuggers. To work around this:
					// - Set the breakpoint on the PREVIOUS line (line - 1)
					// - When that line finishes and we pause, the NEXT line (user's target) hasn't run yet
					//
					// Line number conversion:
					// - User line N (1-indexed from DAP/Monaco)
					// - After debugger injection: user line N -> temp file line N+1 (1-indexed)
					// - WebKit uses 0-indexed: temp file line N+1 (1-indexed) = line N (0-indexed)
					// - Subtract 1 for the "after execution" workaround: line N-1 (0-indexed)
					//
					// Edge case: if line <= 1, we can't go earlier, set at line 0 (our debugger statement)
					const adjustedLine = line > 1 ? line - 1 : 0

					logger.info(`Setting breakpoint: user line ${line} -> adjusted lineNumber ${adjustedLine} (workaround for after-exec pause), urlRegex: ${urlRegex}`)
					const response = await this.sendInspectorCommand('Debugger.setBreakpointByUrl', {
						lineNumber: adjustedLine,
						urlRegex,
						columnNumber: 0
					})
					logger.info(`Breakpoint response: ${JSON.stringify(response)}`)
					if (response.result?.breakpointId) {
						ids.push(response.result.breakpointId as string)
						logger.info(`Breakpoint set by URL at user line ${line} (adjusted line ${adjustedLine}, ID: ${response.result.breakpointId})`)
					} else {
						logger.warn(`No breakpointId in response for line ${line}`)
					}
				} catch (error) {
					logger.error(`Failed to set breakpoint at line ${line}:`, error)
				}
			}
			this.breakpointIds.set(filePath, ids)
		}
		logger.info(`Finished setting breakpoints. Total IDs: ${[...this.breakpointIds.values()].flat().length}`)
	}

	private async handlePaused(params: Record<string, unknown>): Promise<void> {
		const callFrames = params.callFrames as V8CallFrame[] | undefined
		const reason = params.reason as string

		logger.info(`handlePaused called: reason=${reason}, lineNumber=${callFrames?.[0]?.location?.lineNumber}, scriptId=${callFrames?.[0]?.location?.scriptId}`)

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

		logger.info(`isInitialPause=${isInitialPause}, initialPauseDone=${this.initialPauseDone}`)

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
		// With injected debugger at line 0, inspector line N = user line N
		// (0-indexed N + 1 for 1-indexed - 1 for injected debugger = N)
		//
		// For breakpoints: we set the breakpoint on line N-1 due to WebKit's after-exec behavior,
		// so we add 1 to report the correct user line (the one about to execute).
		// For stepping: report the actual line (the one just executed).
		this.pausedAtBreakpoint = dapReason === 'breakpoint'
		let line: number | undefined
		if (callFrames && callFrames.length > 0) {
			const inspectorLine = callFrames[0].location.lineNumber
			// For breakpoint pauses, add 1 because we set BP on line-1
			if (this.pausedAtBreakpoint) {
				line = inspectorLine + 1
			} else {
				line = inspectorLine
			}
		}

		this.sendEvent('stopped', {
			reason: dapReason,
			threadId: 1,
			allThreadsStopped: true,
			line
		})

		this.isStepping = false
		for (const outputEvent of this.pendingConsoleOutput) {
			this.sendEvent('output', outputEvent)
		}
		this.pendingConsoleOutput = []
	}

	private handleConsoleMessageAdded(params: Record<string, unknown>): void {
		const message = params.message as {
			level?: string
			text?: string
			line?: number
			column?: number
			url?: string
			parameters?: Array<{ type: string; value?: unknown; description?: string }>
		}

		if (!message) return

		let output: string
		if (message.parameters && message.parameters.length > 0) {
			output = message.parameters
				.map((p) => (p.value !== undefined ? String(p.value) : p.description || ''))
				.join(' ')
		} else {
			output = message.text || ''
		}

		if (output.startsWith('__WINDMILL_RESULT__:')) {
			try {
				const resultJson = output.substring('__WINDMILL_RESULT__:'.length)
				this.scriptResult = JSON.parse(resultJson)
				logger.info(`Captured script result: ${resultJson}`)

				this.isStepping = false
				for (const pendingOutput of this.pendingConsoleOutput) {
					this.sendEvent('output', pendingOutput)
				}
				this.pendingConsoleOutput = []

				if (!this.terminatedSent) {
					this.terminatedSent = true
					this.running = false
					this.sendEvent('terminated', { result: this.scriptResult })
				}
			} catch (error) {
				logger.error('Failed to parse script result:', error)
			}
			return
		}

		const category = message.level === 'error' || message.level === 'warning' ? 'stderr' : 'stdout'
		const outputEvent: Record<string, unknown> = {
			category,
			output: output + '\n'
		}

		if (message.line !== undefined && message.url) {
			outputEvent.source = {
				path: message.url,
				name: message.url.split('/').pop() || 'script.ts'
			}
			outputEvent.line = message.line
			if (message.column !== undefined) {
				outputEvent.column = message.column + 1
			}
		}

		if (this.isStepping) {
			this.pendingConsoleOutput.push(outputEvent)
		} else {
			this.sendEvent('output', outputEvent)
		}
	}

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

	private async startBunProcess(cwd: string): Promise<void> {
		if (!this.scriptPath) {
			throw new Error('No script path')
		}

		const inspectPort = 9229 + Math.floor(Math.random() * 1000)
		const inspectUrl = `127.0.0.1:${inspectPort}`

		const wsUrlPromise = new Promise<string>((resolve, reject) => {
			this.inspectorWsUrlPromise = { resolve, reject }
			setTimeout(() => {
				if (this.inspectorWsUrlPromise) {
					reject(new Error('Timeout waiting for inspector WebSocket URL'))
					this.inspectorWsUrlPromise = null
				}
			}, 10000)
		})

		// Use --inspect-wait to wait for debugger connection
		this.process = spawnProcess({
			cmd: [config.bunPath, `--inspect-wait=${inspectUrl}`, this.scriptPath],
			cwd,
			env: { NODE_ENV: 'development' }
		})

		this.readStderrForInspectorUrl(this.process.stderr)

		const wsUrl = await wsUrlPromise
		await this.connectToInspector(wsUrl)

		this.process.exited.then(async (exitCode) => {
			logger.info(`Bun process exited with code: ${exitCode}`)
			this.running = false

			if (!this.terminatedSent) {
				this.terminatedSent = true
				try {
					this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
				} catch (error) {
					logger.error('Failed to send terminated event:', error)
				}
			}

			await this.cleanup()
		})
	}

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

				const wsMatch = buffer.match(/ws:\/\/[\d.]+:\d+\/[a-z0-9]+/i)
				if (wsMatch && this.inspectorWsUrlPromise) {
					this.inspectorWsUrl = wsMatch[0]
					this.inspectorWsUrlPromise.resolve(wsMatch[0])
					this.inspectorWsUrlPromise = null
				}

				if (!text.includes('Bun Inspector') && !text.includes('Listening:') &&
					!text.includes('ws://') && !text.includes('debug.bun.sh')) {
					this.sendEvent('output', { category: 'stderr', output: text })
				}
			}
		} catch (error) {
			logger.debug('Stderr stream ended:', error)
		}
	}

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
					// Reset breakpoints state for new session
					this.breakpointsSet = false
					this.breakpointsSetResolve = null

					await this.sendInspectorCommand('Inspector.enable', {})
					await this.sendInspectorCommand('Console.enable', {})
					await this.sendInspectorCommand('Debugger.enable', {})
					await this.sendInspectorCommand('Runtime.enable', {})
					await this.sendInspectorCommand('Debugger.setBreakpointsActive', { active: true })
					await this.sendInspectorCommand('Debugger.setPauseOnDebuggerStatements', { enabled: true })
					await this.sendInspectorCommand('Debugger.setPauseOnExceptions', { state: 'uncaught' })

					// Apply breakpoints by URL pattern (works before script is fully parsed)
					await this.applyBreakpointsByUrl()

					this.running = true
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

				if (!this.terminatedSent) {
					this.terminatedSent = true
					this.running = false
					try {
						this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
					} catch (error) {
						logger.error('Failed to send terminated event on inspector close:', error)
					}
				}
			}
		})
	}

	async handleRequest(request: DAPMessage): Promise<void> {
		const command = request.command || ''

		switch (command) {
			case 'initialize':
				await this.handleInitialize(request)
				break
			case 'setBreakpoints':
				await this.handleSetBreakpoints(request)
				break
			case 'configurationDone':
				this.configured = true
				this.sendResponse(request)
				break
			case 'launch':
				await this.handleLaunch(request)
				break
			case 'threads':
				this.sendResponse(request, true, { threads: [{ id: 1, name: 'MainThread' }] })
				break
			case 'stackTrace':
				await this.handleStackTrace(request)
				break
			case 'scopes':
				await this.handleScopes(request)
				break
			case 'variables':
				await this.handleVariables(request)
				break
			case 'evaluate':
				await this.handleEvaluate(request)
				break
			case 'continue':
				await this.handleContinue(request)
				break
			case 'next':
				await this.handleNext(request)
				break
			case 'stepIn':
				await this.handleStepIn(request)
				break
			case 'stepOut':
				await this.handleStepOut(request)
				break
			case 'pause':
				await this.handlePause(request)
				break
			case 'disconnect':
			case 'terminate':
				await this.handleTerminate(request)
				break
			default:
				this.sendResponse(request, false, {}, `Unsupported command: ${command}`)
		}
	}

	private async handleInitialize(request: DAPMessage): Promise<void> {
		this.sendResponse(request, true, {
			supportsConfigurationDoneRequest: true,
			supportsEvaluateForHovers: true,
			supportTerminateDebuggee: true,
			supportsTerminateRequest: true
		})
		this.initialized = true
		this.sendEvent('initialized')
	}

	private async handleSetBreakpoints(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const source = args.source as { path?: string } | undefined
		const sourcePath = source?.path || ''
		const breakpointsData = (args.breakpoints as Array<{ line: number }>) || []

		const verifiedBreakpoints: Array<{ id: number; verified: boolean; line: number; source?: { path: string } }> = []
		const lineNumbers: number[] = []

		for (const bp of breakpointsData) {
			lineNumbers.push(bp.line)
			verifiedBreakpoints.push({
				id: verifiedBreakpoints.length + 1,
				verified: true,
				line: bp.line,
				source: { path: sourcePath }
			})
		}

		this.breakpoints.set(sourcePath, lineNumbers)

		if (this.inspectorWs) {
			await this.applyBreakpointsByUrl()
		}

		this.sendResponse(request, true, { breakpoints: verifiedBreakpoints })
	}

	private async handleLaunch(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		let code = args.code as string | undefined
		this.scriptPath = args.program as string | undefined
		const cwd = (args.cwd as string) || process.cwd()
		this.callMain = (args.callMain as boolean) || false
		this.mainArgs = (args.args as Record<string, unknown>) || {}

		// Debug: log received code
		const codeLines = code?.split('\n').length ?? 0
		logger.info(`[LAUNCH] Received code with ${codeLines} lines`)
		logger.info(`[LAUNCH] Code preview (first 300 chars): ${code?.substring(0, 300)}`)
		logger.info(`[LAUNCH] callMain=${this.callMain}, args=${JSON.stringify(this.mainArgs)}`)

		if (!this.scriptPath && !code) {
			this.sendResponse(request, false, {}, 'No program or code specified')
			return
		}

		// Reset state for new launch
		this.initialPauseDone = false

		// Inject a debugger statement at the beginning to force initial pause
		// This gives us a known pause point where we can ensure breakpoints are set
		code = `debugger; // Auto-injected by Windmill debugger\n${code || ''}`

		if (this.callMain && code) {
			const argsValues = Object.values(this.mainArgs)
				.map((v) => JSON.stringify(v))
				.join(', ')
			code +=
				`\n\n// Auto-generated call to main entrypoint\n` +
				`globalThis.__windmill_result__ = await main(${argsValues});\n` +
				`console.log("__WINDMILL_RESULT__:" + JSON.stringify(globalThis.__windmill_result__));\n` +
				`await new Promise(r => setTimeout(r, 50));\n`
		}

		if (code && !this.scriptPath) {
			try {
				this.tempDir = await mkdtemp(join(tmpdir(), 'windmill_debug_'))
				this.tempFile = join(this.tempDir, 'script.ts')
				await writeFile(this.tempFile, code)
				this.scriptPath = this.tempFile
			} catch (error) {
				this.sendResponse(request, false, {}, `Failed to create temp file: ${error}`)
				return
			}
		}

		this.sendResponse(request)

		try {
			await this.startBunProcess(cwd)
		} catch (error) {
			this.sendEvent('output', { category: 'stderr', output: `Failed to start Bun: ${error}\n` })
			this.sendEvent('terminated', { error: String(error) })
		}
	}

	private async handleStackTrace(request: DAPMessage): Promise<void> {
		logger.info(`handleStackTrace: callFrames.length=${this.callFrames.length}`)
		const stackFrames: Array<{ id: number; name: string; source: { path: string; name: string }; line: number; column: number }> = []

		for (let i = 0; i < this.callFrames.length; i++) {
			const frame = this.callFrames[i]
			const script = this.scripts.get(frame.location.scriptId)

			logger.info(`  frame ${i}: functionName="${frame.functionName}", scriptId=${frame.location.scriptId}, lineNumber=${frame.location.lineNumber}, scriptUrl=${script?.url}`)

			if (!script?.url || !this.scriptPath) {
				logger.info(`  skipping frame ${i}: no script URL or scriptPath`)
				continue
			}

			const isUserScript = script.url.endsWith(this.scriptPath.split('/').pop()!)
			if (!isUserScript) {
				logger.info(`  skipping frame ${i}: not from user script (${script.url})`)
				continue
			}

			// Inspector line is 0-indexed. With injected debugger at line 0:
			// Inspector 0-indexed line N = user's 1-indexed line N
			// (0-indexed N + 1 to convert to 1-indexed - 1 for injected debugger = N)
			//
			// For breakpoint pauses: we set BP on line N-1, so add 1 to the top frame
			// to show the correct line (about to execute)
			let frameLine = frame.location.lineNumber
			if (i === 0 && this.pausedAtBreakpoint) {
				frameLine = frameLine + 1
			}

			stackFrames.push({
				id: i + 1,
				name: frame.functionName || '<module>',
				source: {
					path: script.url || this.scriptPath || '<unknown>',
					name: script.url?.split('/').pop() || 'script.ts'
				},
				line: frameLine,
				column: frame.location.columnNumber + 1
			})
		}

		logger.info(`handleStackTrace: returning ${stackFrames.length} frames`)
		this.sendResponse(request, true, { stackFrames, totalFrames: stackFrames.length })
	}

	private async handleScopes(request: DAPMessage): Promise<void> {
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
			const ref = this.nextVarRef()
			const objectId = scope.object.objectId

			if (objectId) {
				this.scopesMap.set(ref, { type: scope.type, objectId, frameIndex })
				let name = scope.name || scope.type.charAt(0).toUpperCase() + scope.type.slice(1)
				scopes.push({
					name,
					variablesReference: ref,
					expensive: scope.type === 'global'
				})
			}
		}

		this.sendResponse(request, true, { scopes })
	}

	private async handleVariables(request: DAPMessage): Promise<void> {
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

			const variables: Array<{ name: string; value: string; type: string; variablesReference: number }> = []
			const properties = (response.result?.properties as Array<{
				name: string
				value?: { type: string; value?: unknown; description?: string; objectId?: string; subtype?: string }
			}>) || []

			const builtInNames = new Set([
				'NaN', 'Infinity', 'undefined', 'globalThis', 'global', 'self', 'window',
				'console', 'Bun', 'process', 'navigator', 'performance', 'crypto', 'Loader',
				'Object', 'Array', 'Function', 'Boolean', 'Symbol', 'Number', 'BigInt',
				'String', 'RegExp', 'Date', 'Promise', 'Map', 'Set', 'WeakMap', 'WeakSet',
				'Error', 'TypeError', 'RangeError', 'SyntaxError', 'ReferenceError'
			])

			for (const prop of properties) {
				if (!prop.value) continue
				if (prop.name.startsWith('__') && prop.name !== '__windmill_result__') continue
				if (prop.value.type === 'function' && prop.value.description?.includes('[native code]')) continue
				if (builtInNames.has(prop.name)) continue
				if (prop.value.type === 'function' && /^[A-Z][a-zA-Z0-9]*$/.test(prop.name)) continue

				let value: string
				let varRef = 0
				let displayType = prop.value.type

				if (prop.value.type === 'object' && prop.value.objectId) {
					varRef = this.nextVarRef()
					this.objectsMap.set(varRef, prop.value.objectId)
					value = prop.value.description || '[Object]'
				} else if (prop.value.type === 'string') {
					value = prop.value.value !== undefined ? JSON.stringify(prop.value.value) : (prop.value.description || '""')
				} else if (prop.value.value !== undefined) {
					value = JSON.stringify(prop.value.value)
				} else {
					value = prop.value.description || String(prop.value.type)
				}

				variables.push({ name: prop.name, value, type: displayType, variablesReference: varRef })
			}

			this.sendResponse(request, true, { variables })
		} catch (error) {
			logger.error('Failed to get variables:', error)
			this.sendResponse(request, true, { variables: [] })
		}
	}

	private async handleEvaluate(request: DAPMessage): Promise<void> {
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

				const evalResult = response.result?.result as { type: string; value?: unknown; description?: string; objectId?: string }
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
				const evalResult = response.result?.result as { type: string; value?: unknown; description?: string }
				result = evalResult?.value !== undefined ? JSON.stringify(evalResult.value) : (evalResult?.description || 'undefined')
			}

			this.sendResponse(request, true, { result, variablesReference: varRef })
		} catch (error) {
			this.sendResponse(request, true, { result: `Error: ${error}`, variablesReference: 0 })
		}
	}

	private async handleContinue(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false
			await this.sendInspectorCommand('Debugger.resume', {})
			this.sendResponse(request, true, { allThreadsContinued: true })
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handleNext(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false
			await this.sendInspectorCommand('Debugger.stepOver', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handleStepIn(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false
			await this.sendInspectorCommand('Debugger.stepInto', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handleStepOut(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false
			await this.sendInspectorCommand('Debugger.stepOut', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handlePause(request: DAPMessage): Promise<void> {
		try {
			await this.sendInspectorCommand('Debugger.pause', {})
			this.sendResponse(request)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handleTerminate(request: DAPMessage): Promise<void> {
		this.running = false
		const shouldSendTerminated = !this.terminatedSent
		this.terminatedSent = true
		await this.cleanup()
		this.sendResponse(request)
		if (shouldSendTerminated) {
			this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
		}
	}

	async cleanup(): Promise<void> {
		if (this.inspectorWs) {
			this.inspectorWs.close()
			this.inspectorWs = null
		}

		if (this.process) {
			this.process.kill()
			this.process = null
		}

		if (this.tempFile) {
			try {
				await unlink(this.tempFile)
			} catch {}
			this.tempFile = null
		}

		if (this.tempDir) {
			try {
				await rmdir(this.tempDir)
			} catch {}
			this.tempDir = null
		}
	}
}

// ============================================================================
// Python Debug Session
// ============================================================================

class PythonDebugSession extends BaseDebugSession {
	private debugpyWs: WebSocket | null = null
	private debugpySeq = 1
	private pendingDebugpyRequests = new Map<
		number,
		{ resolve: (value: DAPMessage) => void; reject: (error: Error) => void }
	>()
	private callFrames: Array<{ id: number; name: string; line: number; column: number; source?: { path: string; name?: string } }> = []
	private variablesRefCounter = 1
	private scopesMap = new Map<number, { variablesReference: number }>()
	private scriptResult: unknown = undefined

	private nextDebugpySeq(): number {
		return this.debugpySeq++
	}

	private nextVarRef(): number {
		return this.variablesRefCounter++
	}

	private async sendDebugpyRequest(command: string, args?: Record<string, unknown>): Promise<DAPMessage> {
		if (!this.debugpyWs || this.debugpyWs.readyState !== WebSocket.OPEN) {
			throw new Error('Debugpy not connected')
		}

		const seq = this.nextDebugpySeq()
		const message: DAPMessage = {
			seq,
			type: 'request',
			command,
			arguments: args
		}

		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pendingDebugpyRequests.delete(seq)
				reject(new Error(`Debugpy command timeout: ${command}`))
			}, 10000)

			this.pendingDebugpyRequests.set(seq, {
				resolve: (value) => {
					clearTimeout(timeout)
					resolve(value)
				},
				reject: (error) => {
					clearTimeout(timeout)
					reject(error)
				}
			})

			logger.debug('Sending to debugpy:', JSON.stringify(message))
			this.debugpyWs!.send(JSON.stringify(message))
		})
	}

	private handleDebugpyMessage(data: string): void {
		try {
			const message: DAPMessage = JSON.parse(data)
			logger.debug('Debugpy message:', data.substring(0, 200))

			if (message.type === 'response') {
				const pending = this.pendingDebugpyRequests.get(message.request_seq!)
				if (pending) {
					this.pendingDebugpyRequests.delete(message.request_seq!)
					if (message.success) {
						pending.resolve(message)
					} else {
						pending.reject(new Error(message.message || 'Request failed'))
					}
				}
			} else if (message.type === 'event') {
				this.handleDebugpyEvent(message)
			}
		} catch (error) {
			logger.error('Failed to parse debugpy message:', error)
		}
	}

	private handleDebugpyEvent(event: DAPMessage): void {
		const body = event.body || {}

		switch (event.event) {
			case 'initialized':
				this.sendEvent('initialized')
				break
			case 'stopped':
				this.sendEvent('stopped', {
					reason: body.reason || 'breakpoint',
					threadId: body.threadId || 1,
					allThreadsStopped: body.allThreadsStopped ?? true,
					line: body.line
				})
				break
			case 'continued':
				this.sendEvent('continued', { threadId: body.threadId || 1 })
				break
			case 'terminated':
				if (!this.terminatedSent) {
					this.terminatedSent = true
					// Include captured result in terminated event
					this.sendEvent('terminated', { ...body, result: this.scriptResult })
				}
				break
			case 'output': {
				// Capture the result from __WINDMILL_RESULT__ output
				const output = body.output as string | undefined
				if (output && output.startsWith('__WINDMILL_RESULT__:')) {
					try {
						const resultJson = output.substring('__WINDMILL_RESULT__:'.length).trim()
						this.scriptResult = JSON.parse(resultJson)
						logger.info(`Python: Captured script result: ${resultJson}`)
					} catch (error) {
						logger.error('Failed to parse Python result:', error)
					}
					// Don't forward __WINDMILL_RESULT__ output to client
					break
				}
				this.sendEvent('output', body)
				break
			}
			case 'exited':
				if (!this.terminatedSent) {
					this.terminatedSent = true
					this.sendEvent('terminated', { result: this.scriptResult })
				}
				break
		}
	}

	private async startPythonProcess(cwd: string): Promise<void> {
		if (!this.scriptPath) {
			throw new Error('No script path')
		}

		const debugpyPort = 5678 + Math.floor(Math.random() * 1000)

		// Get the directory where this script is located to find dap_websocket_server.py
		const scriptDir = import.meta.dir

		// Spawn the Python WebSocket DAP server
		this.process = spawnProcess({
			cmd: [
				config.pythonPath,
				'-u',
				join(scriptDir, 'dap_websocket_server.py'),
				'--port', String(debugpyPort),
				'--host', '127.0.0.1'
			],
			cwd,
			env: { PYTHONUNBUFFERED: '1' }
		})

		// Read stderr to capture startup messages
		this.readPythonStderr(this.process.stderr)

		// Wait for the Python server to be ready (check via health endpoint or just wait)
		await this.waitForPythonServer(debugpyPort)

		// Connect to the Python WebSocket server
		await this.connectToDebugpy(`ws://127.0.0.1:${debugpyPort}`)

		this.process.exited.then(async (exitCode) => {
			logger.info(`Python process exited with code: ${exitCode}`)
			this.running = false

			if (!this.terminatedSent) {
				this.terminatedSent = true
				this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
			}

			await this.cleanup()
		})
	}

	private async readPythonStderr(stream: ReadableStream<Uint8Array> | null): Promise<void> {
		if (!stream) return

		const reader = stream.getReader()
		const decoder = new TextDecoder()

		try {
			while (true) {
				const { done, value } = await reader.read()
				if (done) break

				const text = decoder.decode(value)
				// Forward stderr output to the client
				if (text.trim()) {
					logger.debug('Python stderr:', text)
				}
			}
		} catch (error) {
			logger.debug('Python stderr stream ended:', error)
		}
	}

	private async waitForPythonServer(port: number, maxAttempts = 20): Promise<void> {
		for (let i = 0; i < maxAttempts; i++) {
			try {
				// Try to connect to see if server is up
				const testWs = new WebSocket(`ws://127.0.0.1:${port}`)
				await new Promise<void>((resolve, reject) => {
					const timeout = setTimeout(() => {
						testWs.close()
						reject(new Error('Connection timeout'))
					}, 500)

					testWs.onopen = () => {
						clearTimeout(timeout)
						testWs.close()
						resolve()
					}
					testWs.onerror = () => {
						clearTimeout(timeout)
						reject(new Error('Connection failed'))
					}
				})
				logger.info(`Python server ready on port ${port}`)
				return
			} catch {
				await new Promise(resolve => setTimeout(resolve, 100))
			}
		}
		throw new Error('Python server did not start in time')
	}

	private async connectToDebugpy(wsUrl: string): Promise<void> {
		logger.info(`Connecting to debugpy at ${wsUrl}`)

		return new Promise((resolve, reject) => {
			this.debugpyWs = new WebSocket(wsUrl)

			const timeout = setTimeout(() => {
				reject(new Error('Debugpy connection timeout'))
			}, 5000)

			this.debugpyWs.onopen = async () => {
				clearTimeout(timeout)
				logger.info('Connected to debugpy')

				try {
					await this.sendDebugpyRequest('initialize', {
						clientID: 'windmill',
						clientName: 'Windmill Debug Service',
						adapterID: 'python',
						pathFormat: 'path',
						linesStartAt1: true,
						columnsStartAt1: true
					})
					this.running = true
					resolve()
				} catch (error) {
					reject(error)
				}
			}

			this.debugpyWs.onmessage = (event) => {
				this.handleDebugpyMessage(event.data as string)
			}

			this.debugpyWs.onerror = (error) => {
				logger.error('Debugpy WebSocket error:', error)
			}

			this.debugpyWs.onclose = () => {
				logger.info('Debugpy WebSocket closed')
				this.debugpyWs = null
			}
		})
	}

	async handleRequest(request: DAPMessage): Promise<void> {
		const command = request.command || ''

		switch (command) {
			case 'initialize':
				await this.handleInitialize(request)
				break
			case 'setBreakpoints':
				await this.handleSetBreakpoints(request)
				break
			case 'configurationDone':
				this.configured = true
				if (this.debugpyWs) {
					await this.sendDebugpyRequest('configurationDone')
				}
				this.sendResponse(request)
				break
			case 'launch':
				await this.handleLaunch(request)
				break
			case 'threads':
				await this.forwardToDebugpy(request)
				break
			case 'stackTrace':
				await this.forwardToDebugpy(request)
				break
			case 'scopes':
				await this.forwardToDebugpy(request)
				break
			case 'variables':
				await this.forwardToDebugpy(request)
				break
			case 'evaluate':
				await this.forwardToDebugpy(request)
				break
			case 'continue':
				await this.forwardToDebugpy(request)
				break
			case 'next':
				await this.forwardToDebugpy(request)
				break
			case 'stepIn':
				await this.forwardToDebugpy(request)
				break
			case 'stepOut':
				await this.forwardToDebugpy(request)
				break
			case 'pause':
				await this.forwardToDebugpy(request)
				break
			case 'disconnect':
			case 'terminate':
				await this.handleTerminate(request)
				break
			default:
				this.sendResponse(request, false, {}, `Unsupported command: ${command}`)
		}
	}

	private async forwardToDebugpy(request: DAPMessage): Promise<void> {
		try {
			const response = await this.sendDebugpyRequest(request.command!, request.arguments)
			this.sendResponse(request, response.success, response.body, response.message)
		} catch (error) {
			this.sendResponse(request, false, {}, String(error))
		}
	}

	private async handleInitialize(request: DAPMessage): Promise<void> {
		this.sendResponse(request, true, {
			supportsConfigurationDoneRequest: true,
			supportsEvaluateForHovers: true,
			supportTerminateDebuggee: true,
			supportsTerminateRequest: true
		})
		this.initialized = true
	}

	private async handleSetBreakpoints(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const source = args.source as { path?: string } | undefined
		const sourcePath = source?.path || ''
		const breakpointsData = (args.breakpoints as Array<{ line: number }>) || []

		const lineNumbers = breakpointsData.map(bp => bp.line)
		this.breakpoints.set(sourcePath, lineNumbers)

		if (this.debugpyWs) {
			try {
				const response = await this.sendDebugpyRequest('setBreakpoints', args)
				this.sendResponse(request, response.success, response.body)
			} catch (error) {
				this.sendResponse(request, false, {}, String(error))
			}
		} else {
			// Debugpy not connected yet, respond with unverified breakpoints
			const breakpoints = lineNumbers.map((line, i) => ({
				id: i + 1,
				verified: false,
				line,
				source: { path: sourcePath }
			}))
			this.sendResponse(request, true, { breakpoints })
		}
	}

	private async handleLaunch(request: DAPMessage): Promise<void> {
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

		// If callMain is true, wrap the code to call main() with args
		if (this.callMain && code) {
			const argsJson = JSON.stringify(this.mainArgs)
			code += `

# Auto-generated call to main entrypoint
import json
import sys
_args = json.loads('${argsJson.replace(/'/g, "\\'")}')
_result = main(**_args)
print("__WINDMILL_RESULT__:" + json.dumps(_result))
sys.stdout.flush()
`
		}

		if (code && !this.scriptPath) {
			try {
				this.tempDir = await mkdtemp(join(tmpdir(), 'windmill_debug_'))
				this.tempFile = join(this.tempDir, 'script.py')
				await writeFile(this.tempFile, code)
				this.scriptPath = this.tempFile
			} catch (error) {
				this.sendResponse(request, false, {}, `Failed to create temp file: ${error}`)
				return
			}
		}

		this.sendResponse(request)

		try {
			await this.startPythonProcess(cwd)

			// Re-apply breakpoints to the Python server using the actual script path
			for (const [, lines] of this.breakpoints) {
				await this.sendDebugpyRequest('setBreakpoints', {
					source: { path: this.scriptPath },
					breakpoints: lines.map(line => ({ line }))
				})
			}

			// Signal configuration done
			await this.sendDebugpyRequest('configurationDone')

			// Now forward the launch command to the Python server
			// The Python server needs to know what code/program to debug
			await this.sendDebugpyRequest('launch', {
				program: this.scriptPath,
				code: code,
				args: this.mainArgs,
				cwd: cwd,
				callMain: this.callMain
			})
		} catch (error) {
			this.sendEvent('output', { category: 'stderr', output: `Failed to start Python: ${error}\n` })
			this.sendEvent('terminated', { error: String(error) })
		}
	}

	private async handleTerminate(request: DAPMessage): Promise<void> {
		this.running = false
		const shouldSendTerminated = !this.terminatedSent
		this.terminatedSent = true

		if (this.debugpyWs) {
			try {
				await this.sendDebugpyRequest('terminate')
			} catch {}
		}

		await this.cleanup()
		this.sendResponse(request)

		if (shouldSendTerminated) {
			this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
		}
	}

	async cleanup(): Promise<void> {
		if (this.debugpyWs) {
			this.debugpyWs.close()
			this.debugpyWs = null
		}

		if (this.process) {
			this.process.kill()
			this.process = null
		}

		if (this.tempFile) {
			try {
				await unlink(this.tempFile)
			} catch {}
			this.tempFile = null
		}

		if (this.tempDir) {
			try {
				await rmdir(this.tempDir)
			} catch {}
			this.tempDir = null
		}
	}
}

// ============================================================================
// Main Server
// ============================================================================

const sessions = new Map<unknown, BaseDebugSession>()

logger.info(`Starting DAP Debug Service on ${config.host}:${config.port}`)
logger.info(`Endpoints: /python, /typescript, /bun`)
if (config.nsjail.enabled) {
	logger.info(`nsjail enabled: ${config.nsjail.binaryPath}`)
	if (config.nsjail.configPath) {
		logger.info(`nsjail config: ${config.nsjail.configPath}`)
	}
}

const server = Bun.serve({
	hostname: config.host,
	port: config.port,
	fetch(req, server) {
		const url = new URL(req.url)
		const path = url.pathname

		// Handle WebSocket upgrade with path-based routing
		if (server.upgrade(req, { data: { path } })) {
			return undefined as unknown as Response
		}

		// Health check endpoint
		if (path === '/health') {
			return new Response(JSON.stringify({
				status: 'ok',
				endpoints: ['/python', '/typescript', '/bun'],
				nsjail: config.nsjail.enabled
			}), {
				headers: { 'Content-Type': 'application/json' }
			})
		}

		return new Response('DAP Debug Service\n\nEndpoints:\n  /python - Python debugging\n  /typescript - TypeScript/Bun debugging\n  /bun - TypeScript/Bun debugging\n  /health - Health check', {
			status: 200
		})
	},
	websocket: {
		open(ws) {
			const path = (ws.data as { path: string }).path
			logger.info(`New client connected: ${path}`)

			// Create appropriate session based on path
			const wsWrapper = {
				send: (data: string) => ws.send(data),
				close: () => ws.close()
			}

			let session: BaseDebugSession

			if (path === '/python') {
				session = new PythonDebugSession(wsWrapper)
			} else if (path === '/typescript' || path === '/bun' || path === '/') {
				// Use the working Bun debug session from dap_websocket_server_bun.ts
				session = new BunDebugSessionWorking(wsWrapper as unknown as WebSocket) as unknown as BaseDebugSession
			} else {
				logger.warn(`Unknown path: ${path}, defaulting to TypeScript`)
				session = new BunDebugSessionWorking(wsWrapper as unknown as WebSocket) as unknown as BaseDebugSession
			}

			sessions.set(ws, session)
		},
		async message(ws, message) {
			const session = sessions.get(ws)
			if (!session) return

			try {
				const data = JSON.parse(message as string) as DAPMessage
				logger.debug('Received:', JSON.stringify(data).substring(0, 200))

				if (data.type === 'request') {
					await session.handleRequest(data)
				}
			} catch (error) {
				logger.error('Error handling message:', error)
			}
		},
		async close(ws) {
			logger.info('Client disconnected')
			const session = sessions.get(ws)
			if (session) {
				await session.cleanup()
				sessions.delete(ws)
			}
		}
	}
})

logger.info(`Server started on ${server.url}`)

// Handle graceful shutdown
process.on('SIGTERM', async () => {
	logger.info('Received SIGTERM, shutting down...')
	for (const session of sessions.values()) {
		await session.cleanup()
	}
	process.exit(0)
})

process.on('SIGINT', async () => {
	logger.info('Received SIGINT, shutting down...')
	for (const session of sessions.values()) {
		await session.cleanup()
	}
	process.exit(0)
})
