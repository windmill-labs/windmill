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
 *   --port PORT           Server port (default: 3003)
 *   --host HOST           Server host (default: 0.0.0.0)
 *   --nsjail              Enable nsjail wrapping
 *   --nsjail-config PATH  Path to nsjail config file
 *   --windmill PATH       Path to windmill binary for automatic dependency installation
 *   --debug               Enable debug logging
 *
 * Environment Variables:
 *   DAP_PORT              Server port
 *   DAP_HOST              Server host
 *   DAP_NSJAIL_ENABLED    Enable nsjail (true/false)
 *   DAP_NSJAIL_CONFIG     Path to nsjail config file
 *   DAP_NSJAIL_PATH       Path to nsjail binary (default: nsjail)
 *   DAP_WINDMILL_PATH     Path to windmill binary for dependency auto-installation
 *   DAP_DEBUG             Enable debug logging (true/false)
 */

import { spawn, type Subprocess } from 'bun'
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

// Import the working Bun debug session from the standalone server
import { DebugSession as BunDebugSessionWorking, type NsjailConfig } from './dap_websocket_server_bun'

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
	// Windmill binary path for prepare-deps CLI (optional, for dependency auto-installation)
	windmillPath?: string
	// Enable debug logging
	debug: boolean
}

function parseConfig(): ServiceConfig {
	const args = process.argv.slice(2)
	const config: ServiceConfig = {
		port: parseInt(process.env.DAP_PORT || '3003', 10),
		host: process.env.DAP_HOST || '0.0.0.0',
		nsjail: {
			enabled: process.env.DAP_NSJAIL_ENABLED === 'true',
			configPath: process.env.DAP_NSJAIL_CONFIG,
			binaryPath: process.env.DAP_NSJAIL_PATH || 'nsjail',
			extraArgs: []
		},
		pythonPath: process.env.DAP_PYTHON_PATH || '/usr/bin/python3',
		bunPath: process.env.DAP_BUN_PATH || '/usr/bin/bun',
		windmillPath: process.env.DAP_WINDMILL_PATH,
		debug: process.env.DAP_DEBUG === 'true'
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
			case '--windmill':
				config.windmillPath = args[++i]
				break
			case '--debug':
				config.debug = true
				break
		}
	}

	return config
}

const config = parseConfig()

// Validate windmill path exists if specified
if (config.windmillPath && !existsSync(config.windmillPath)) {
	console.error(`ERROR: Windmill binary not found at: ${config.windmillPath}`)
	console.error('Please provide a valid path with --windmill /path/to/windmill')
	process.exit(1)
}

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
// JWT Token Verification
// ============================================================================

// JWT verification for debug requests
// The debugger fetches the public key from the Windmill backend's JWKS endpoint
const WINDMILL_BASE_URL = process.env.WINDMILL_BASE_URL || process.env.BASE_INTERNAL_URL // e.g., http://localhost:8000
const REQUIRE_SIGNED_REQUESTS = process.env.REQUIRE_SIGNED_DEBUG_REQUESTS !== 'false'

interface JWK {
	kty: string
	crv: string
	x: string
	kid: string
	use: string
	alg: string
}

interface JWKS {
	keys: JWK[]
}

interface DebugTokenClaims {
	code_hash: string
	language: string
	workspace_id: string
	email: string
	iat: number
	exp: number
	job_id: string
}

// Cached public key
let cachedPublicKey: CryptoKey | null = null
let publicKeyFetchPromise: Promise<CryptoKey | null> | null = null

/**
 * Fetch and cache the Ed25519 public key from the JWKS endpoint.
 */
async function getPublicKey(): Promise<CryptoKey | null> {
	if (cachedPublicKey) {
		return cachedPublicKey
	}

	if (publicKeyFetchPromise) {
		return publicKeyFetchPromise
	}

	if (!WINDMILL_BASE_URL) {
		logger.warn('WINDMILL_BASE_URL not set - cannot fetch public key')
		return null
	}

	publicKeyFetchPromise = (async () => {
		try {
			const jwksUrl = `${WINDMILL_BASE_URL.replace(/\/$/, '')}/api/debug/jwks`
			logger.info(`Fetching JWKS from ${jwksUrl}`)
			const response = await fetch(jwksUrl)
			if (!response.ok) {
				throw new Error(`Failed to fetch JWKS: ${response.status} ${response.statusText}`)
			}

			const jwks: JWKS = await response.json()
			if (!jwks.keys || jwks.keys.length === 0) {
				throw new Error('No keys in JWKS')
			}

			const jwk = jwks.keys[0]
			if (jwk.kty !== 'OKP' || jwk.crv !== 'Ed25519') {
				throw new Error(`Unsupported key type: ${jwk.kty}/${jwk.crv}`)
			}

			// Decode the public key from base64url
			const publicKeyBytes = base64urlDecode(jwk.x)

			// Import as Ed25519 public key
			const key = await crypto.subtle.importKey(
				'raw',
				publicKeyBytes,
				{ name: 'Ed25519' },
				true,
				['verify']
			)

			cachedPublicKey = key
			logger.info('Successfully loaded Ed25519 public key from JWKS')
			return key
		} catch (error) {
			logger.error(`Failed to fetch/parse JWKS: ${error}`)
			return null
		} finally {
			publicKeyFetchPromise = null
		}
	})()

	return publicKeyFetchPromise
}

/**
 * Compute SHA-256 hash of code and return first 16 bytes as hex.
 */
async function computeCodeHash(code: string): Promise<string> {
	const encoder = new TextEncoder()
	const data = encoder.encode(code)
	const hashBuffer = await crypto.subtle.digest('SHA-256', data)
	const hashArray = new Uint8Array(hashBuffer)
	return Array.from(hashArray.slice(0, 16))
		.map(b => b.toString(16).padStart(2, '0'))
		.join('')
}

/**
 * Base64url decode
 */
function base64urlDecode(str: string): Uint8Array {
	// Add padding if needed
	const padding = '='.repeat((4 - str.length % 4) % 4)
	const base64 = str.replace(/-/g, '+').replace(/_/g, '/') + padding
	const binary = atob(base64)
	return Uint8Array.from(binary, c => c.charCodeAt(0))
}

/**
 * Verify a JWT debug token.
 * Returns null if valid, or an error message if invalid.
 */
async function verifyDebugToken(token: string, code: string): Promise<string | null> {
	const publicKey = await getPublicKey()

	if (!publicKey) {
		if (REQUIRE_SIGNED_REQUESTS) {
			return 'Public key not available but signed requests are required. Set WINDMILL_BASE_URL.'
		}
		logger.warn('Public key not available - signature verification disabled')
		return null
	}

	// Parse JWT
	const parts = token.split('.')
	if (parts.length !== 3) {
		return 'Invalid JWT format'
	}

	const [headerB64, claimsB64, signatureB64] = parts

	try {
		// Verify signature
		const message = new TextEncoder().encode(`${headerB64}.${claimsB64}`)
		const signature = base64urlDecode(signatureB64)

		const isValid = await crypto.subtle.verify(
			{ name: 'Ed25519' },
			publicKey,
			signature,
			message
		)

		if (!isValid) {
			return 'Invalid JWT signature'
		}

		// Parse and validate claims
		const claimsJson = new TextDecoder().decode(base64urlDecode(claimsB64))
		const claims: DebugTokenClaims = JSON.parse(claimsJson)

		// Check expiration
		const now = Math.floor(Date.now() / 1000)
		if (now > claims.exp) {
			return `Token expired: ${now - claims.exp} seconds ago`
		}

		// Verify code hash
		const expectedHash = await computeCodeHash(code)
		if (claims.code_hash !== expectedHash) {
			return 'Code hash mismatch - code was modified after signing'
		}

		logger.info(`Verified debug token from ${claims.email} in workspace ${claims.workspace_id} (job: ${claims.job_id})`)
		return null
	} catch (error) {
		return `JWT verification error: ${error}`
	}
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

		// Add working directory if specified
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

	// Only include essential env vars + caller-provided ones
	// Don't inherit all of process.env to keep debugger environment clean
	return spawn({
		cmd,
		cwd: options.cwd || process.cwd(),
		stdout: options.stdout || 'pipe',
		stderr: options.stderr || 'pipe',
		env: {
			// Essential system vars
			PATH: process.env.PATH || '/usr/bin:/bin',
			HOME: process.env.HOME,
			// Caller-provided env vars
			// Note: WM_BASE_URL is already overridden by BASE_INTERNAL_URL if set
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
	private envVars: Record<string, string> = {}
	private windmillPath?: string
	private debugMode: boolean

	constructor(ws: { send: (data: string) => void; close: () => void }, windmillPath?: string, debugMode = false) {
		super(ws)
		this.windmillPath = windmillPath
		this.debugMode = debugMode
	}

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

		// Use a wider port range (10000-60000) to avoid collisions with recently used ports
		const debugpyPort = 10000 + Math.floor(Math.random() * 50000)

		// Get the directory where this script is located to find dap_websocket_server.py
		const scriptDir = import.meta.dir

		// Spawn the Python WebSocket DAP server
		// Pass env vars to the server - it will forward them to the debugged script
		const cmd = [
			config.pythonPath,
			'-u',
			join(scriptDir, 'dap_websocket_server.py'),
			'--port', String(debugpyPort),
			'--host', '127.0.0.1'
		]

		// Pass windmill path for dependency auto-installation if configured
		if (this.windmillPath) {
			cmd.push('--windmill', this.windmillPath)
			logger.info(`Python session: autoinstall enabled with windmill at ${this.windmillPath}`)
		}

		// Pass debug flag to Python subprocess
		if (this.debugMode) {
			cmd.push('--debug')
		}

		this.process = spawnProcess({
			cmd,
			cwd,
			env: { PYTHONUNBUFFERED: '1', ...this.envVars }
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
				// Forward stderr output to the client (Python logs go to stderr)
				if (text.trim()) {
					// Log Python output at info level so we can see it
					for (const line of text.split('\n')) {
						if (line.trim()) {
							logger.info(`[Python] ${line}`)
						}
					}
				}
			}
		} catch (error) {
			logger.debug('Python stderr stream ended:', error)
		}
	}

	private async waitForPythonServer(port: number, maxAttempts = 30): Promise<void> {
		// Wait a bit for Python to start - it takes at least 50-100ms to initialize
		await new Promise(resolve => setTimeout(resolve, 100))

		for (let i = 0; i < maxAttempts; i++) {
			try {
				// Try to connect to see if server is up
				const testWs = new WebSocket(`ws://127.0.0.1:${port}`)
				await new Promise<void>((resolve, reject) => {
					const timeout = setTimeout(() => {
						try { testWs.close() } catch {}
						reject(new Error('Connection timeout'))
					}, 500)

					testWs.onopen = () => {
						clearTimeout(timeout)
						// Wait a moment before closing to ensure the connection is stable
						setTimeout(() => {
							try { testWs.close() } catch {}
							resolve()
						}, 50)
					}
					testWs.onerror = () => {
						clearTimeout(timeout)
						try { testWs.close() } catch {}
						reject(new Error('Connection failed'))
					}
				})
				// Small delay to let the test connection fully close
				await new Promise(resolve => setTimeout(resolve, 50))
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
		let cwd = (args.cwd as string) || process.cwd()
		this.callMain = (args.callMain as boolean) || false
		this.mainArgs = (args.args as Record<string, unknown>) || {}
		this.envVars = (args.env as Record<string, string>) || {}

		// Verify JWT token if code is provided (signed debug request)
		// The token is passed in the launch arguments
		if (code && REQUIRE_SIGNED_REQUESTS) {
			const token = args.token as string | undefined
			if (!token) {
				logger.error('No debug token provided but signed requests are required')
				this.sendResponse(request, false, {}, 'Debug token required. Ensure the debug session was signed by the backend.')
				return
			}

			const verificationError = await verifyDebugToken(token, code)
			if (verificationError) {
				logger.error(`Token verification failed: ${verificationError}`)
				this.sendResponse(request, false, {}, `Token verification failed: ${verificationError}`)
				return
			}
		}

		// If BASE_INTERNAL_URL is set on the server, use it to override WM_BASE_URL
		if (process.env.BASE_INTERNAL_URL) {
			this.envVars.WM_BASE_URL = process.env.BASE_INTERNAL_URL
		}

		if (Object.keys(this.envVars).length > 0) {
			logger.info(`Python launch with env vars: ${Object.keys(this.envVars).join(', ')}`)
		}

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
				// Use temp directory as cwd so debugger can find the script
				cwd = this.tempDir
				logger.info(`Wrote Python code to ${this.tempFile}, cwd=${cwd}`)
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
				callMain: this.callMain,
				env: this.envVars
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
if (config.windmillPath) {
	logger.info(`Windmill binary: ${config.windmillPath} (autoinstall enabled)`)
} else {
	logger.info('Windmill binary: not configured (autoinstall disabled)')
}
if (config.debug) {
	logger.info('Debug logging: enabled')
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
			let path = (ws.data as { path: string }).path

			// Trim /ws_debug prefix if present (for direct access without reverse proxy stripping)
			if (path.startsWith('/ws_debug/')) {
				path = path.slice('/ws_debug'.length)
			}

			logger.info(`New client connected: ${path}`)

			// Create appropriate session based on path
			const wsWrapper = {
				send: (data: string) => ws.send(data),
				close: () => ws.close()
			}

			let session: BaseDebugSession

			// Build nsjail config to pass to debuggers
			const nsjailConfig: NsjailConfig | undefined = config.nsjail.enabled
				? {
						enabled: true,
						binaryPath: config.nsjail.binaryPath,
						configPath: config.nsjail.configPath,
						extraArgs: config.nsjail.extraArgs
					}
				: undefined

			if (path === '/python') {
				session = new PythonDebugSession(wsWrapper, config.windmillPath, config.debug)
			} else if (path === '/typescript' || path === '/bun' || path === '/') {
				// Use the working Bun debug session from dap_websocket_server_bun.ts
				session = new BunDebugSessionWorking(wsWrapper as unknown as WebSocket, {
					nsjailConfig,
					bunPath: config.bunPath,
					windmillPath: config.windmillPath
				}) as unknown as BaseDebugSession
			} else {
				logger.warn(`Unknown path: ${path}, defaulting to TypeScript`)
				session = new BunDebugSessionWorking(wsWrapper as unknown as WebSocket, {
					nsjailConfig,
					bunPath: config.bunPath,
					windmillPath: config.windmillPath
				}) as unknown as BaseDebugSession
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
