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
import { mkdtemp, writeFile, unlink, rmdir, symlink } from 'node:fs/promises'
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
	sourceMapURL?: string
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

interface ExpressionTokenClaims {
	expression_hash: string
	job_id: string
	workspace_id: string
	email: string
	iat: number
	exp: number
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
			const publicKeyBytes = Uint8Array.from(atob(jwk.x.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0))

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

/**
 * Compute SHA-256 hash of an expression and return first 16 bytes as hex.
 */
async function computeExpressionHash(expression: string): Promise<string> {
	const encoder = new TextEncoder()
	const data = encoder.encode(expression)
	const hashBuffer = await crypto.subtle.digest('SHA-256', data)
	const hashArray = new Uint8Array(hashBuffer)
	return Array.from(hashArray.slice(0, 16))
		.map(b => b.toString(16).padStart(2, '0'))
		.join('')
}

/**
 * Verify a JWT expression token.
 * Returns null if valid, or an error message if invalid.
 * Note: Expression tokens are optional - if not provided, the expression is still evaluated
 * but just not audit logged. This maintains backwards compatibility.
 */
async function verifyExpressionToken(token: string, expression: string): Promise<string | null> {
	const publicKey = await getPublicKey()

	if (!publicKey) {
		// Expression tokens are optional - don't block if public key unavailable
		logger.debug('Public key not available - skipping expression token verification')
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
			return 'Invalid expression token signature'
		}

		// Parse and validate claims
		const claimsJson = new TextDecoder().decode(base64urlDecode(claimsB64))
		const claims: ExpressionTokenClaims = JSON.parse(claimsJson)

		// Check expiration
		const now = Math.floor(Date.now() / 1000)
		if (now > claims.exp) {
			return `Expression token expired: ${now - claims.exp} seconds ago`
		}

		// Verify expression hash
		const expectedHash = await computeExpressionHash(expression)
		if (claims.expression_hash !== expectedHash) {
			return 'Expression hash mismatch - expression was modified after signing'
		}

		logger.info(`Verified expression token from ${claims.email} in workspace ${claims.workspace_id} (job: ${claims.job_id})`)
		return null
	} catch (error) {
		return `Expression token verification error: ${error}`
	}
}

/**
 * Remove version specifiers from import statements.
 * Transforms `import x from "package@1.2.3"` to `import x from "package"`
 * This matches the behavior of the Windmill backend's remove_pinned_imports function.
 */
function removePinnedImports(code: string): string {
	// Regex to match import specifiers with version pins
	// Handles both scoped packages (@scope/package@version) and regular packages (package@version)
	// Group 1: package name (including scope if present)
	// The @version part is matched but not captured
	// Group 2: any path suffix after the package (e.g., /subpath)
	const IMPORTS_VERSION = /^((?:@[^/@]+\/[^/@]+)|(?:[^/@]+))(?:@[^/]+)?(.*)$/

	// Find all string literals that look like imports (in quotes)
	// This regex finds strings in import/export statements
	const importRegex = /(?:import|export).*?from\s+['"]([^'"]+)['"]/g

	let result = code
	let match: RegExpExecArray | null

	// Collect all imports first to avoid issues with overlapping replacements
	const imports: string[] = []
	while ((match = importRegex.exec(code)) !== null) {
		imports.push(match[1])
	}

	// Sort by length descending to handle longer matches first (avoids partial replacements)
	imports.sort((a, b) => b.length - a.length)

	// Process each import and remove version specifiers
	for (const importPath of imports) {
		const versionMatch = IMPORTS_VERSION.exec(importPath)
		if (versionMatch) {
			const packageName = versionMatch[1]
			const pathSuffix = versionMatch[2] || ''
			const newImportPath = packageName + pathSuffix

			// Only replace if we actually removed a version
			if (newImportPath !== importPath) {
				// Replace all occurrences of this import in the code
				result = result.split(`"${importPath}"`).join(`"${newImportPath}"`)
				result = result.split(`'${importPath}'`).join(`'${newImportPath}'`)
				logger.debug(`Removed version from import: "${importPath}" -> "${newImportPath}"`)
			}
		}
	}

	return result
}

// Nsjail configuration for sandboxed execution
export interface NsjailConfig {
	enabled: boolean
	binaryPath: string
	configPath?: string
	extraArgs?: string[]
}

/**
 * VLQ (Variable-Length Quantity) decoder for source maps.
 * Returns array of decoded integers from VLQ string.
 */
function decodeVLQ(encoded: string): number[] {
	const VLQ_BASE64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
	const values: number[] = []
	let shift = 0
	let value = 0

	for (const char of encoded) {
		const digit = VLQ_BASE64.indexOf(char)
		if (digit === -1) continue

		const hasContinuation = digit & 32
		value += (digit & 31) << shift

		if (hasContinuation) {
			shift += 5
		} else {
			const isNegative = value & 1
			value = value >> 1
			values.push(isNegative ? -value : value)
			value = 0
			shift = 0
		}
	}
	return values
}

/**
 * Source map line mappings - both directions.
 */
interface SourceMapMappings {
	// original 0-indexed line -> transpiled 0-indexed line (for setting breakpoints)
	originalToTranspiled: Map<number, number>
	// transpiled 0-indexed line -> original 0-indexed line (for reporting positions)
	transpiledToOriginal: Map<number, number>
}

/**
 * Parse source map and build bidirectional line mapping.
 * Returns Maps for both original->transpiled and transpiled->original mappings.
 *
 * Important: When multiple original lines map to the same transpiled line (common with
 * TypeScript transpilation), we store the LAST/HIGHEST original line for the reverse
 * mapping. This is because Bun compresses multi-line function signatures, but the
 * actual statement being executed typically corresponds to the highest original line
 * that maps to that transpiled line.
 */
function parseSourceMapLineMapping(sourceMapUrl: string): SourceMapMappings {
	const mappings: SourceMapMappings = {
		originalToTranspiled: new Map(),
		transpiledToOriginal: new Map()
	}

	try {
		// Extract base64 content from data URL
		const match = sourceMapUrl.match(/^data:application\/json;base64,(.+)$/)
		if (!match) {
			logger.warn('Source map is not a base64 data URL')
			return mappings
		}

		const decoded = atob(match[1])
		const sourceMap = JSON.parse(decoded) as {
			mappings: string
			sources?: string[]
			sourcesContent?: string[]
		}

		if (!sourceMap.mappings) {
			return mappings
		}

		// Parse VLQ mappings
		// Each line in transpiled code is separated by ';'
		// Each segment within a line is separated by ','
		// Segment format: [transpiled column, source index, original line, original column, name index]
		const lines = sourceMap.mappings.split(';')
		let originalLine = 0

		for (let transpiledLine = 0; transpiledLine < lines.length; transpiledLine++) {
			const segments = lines[transpiledLine].split(',')

			for (const segment of segments) {
				if (!segment) continue

				const values = decodeVLQ(segment)
				if (values.length >= 3) {
					// values[2] is the delta for original line
					originalLine += values[2]

					// Map this original line to this transpiled line (for setting breakpoints)
					// Only store the first occurrence (first transpiled line for this original line)
					// This ensures breakpoints are set at the earliest transpiled line containing the original code
					if (!mappings.originalToTranspiled.has(originalLine)) {
						mappings.originalToTranspiled.set(originalLine, transpiledLine)
						logger.debug(`Source map: original line ${originalLine} -> transpiled line ${transpiledLine}`)
					}

					// Map transpiled line to original line (for reporting positions)
					// ALWAYS update to store the HIGHEST original line for each transpiled line.
					// This is critical because when Bun compresses code (e.g., multi-line function
					// params into one line), the actual statement being executed corresponds to
					// the highest original line that maps to that transpiled line.
					// Example: transpiled line 3 might map to original lines 6 and 7;
					// when stopped on transpiled line 3, we want to report line 7 (the return statement)
					// not line 6 (the console.log that happened earlier).
					const existing = mappings.transpiledToOriginal.get(transpiledLine)
					if (existing === undefined || originalLine > existing) {
						mappings.transpiledToOriginal.set(transpiledLine, originalLine)
						logger.debug(`Source map: transpiled line ${transpiledLine} -> original line ${originalLine}`)
					}
				}
			}
		}
	} catch (error) {
		logger.error('Failed to parse source map:', error)
	}

	return mappings
}

/**
 * Manages a debug session with a Bun subprocess.
 */
export class DebugSession {
	private ws: WebSocket
	private seq = 1
	private initialized = false
	private configured = false
	private running = false
	private terminatedSent = false

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

	// Source map line mappings for both directions
	// This is needed because Bun transpiles TypeScript and may strip blank lines/comments
	private sourceMapMappings: SourceMapMappings = {
		originalToTranspiled: new Map(),
		transpiledToOriginal: new Map()
	}

	// Call frames when paused
	private callFrames: V8CallFrame[] = []
	private variablesRefCounter = 1
	private scopesMap = new Map<number, { type: string; objectId: string; frameIndex: number }>()
	private objectsMap = new Map<number, string>() // variablesRef -> objectId

	// For calling main()
	private callMain = false
	private mainArgs: Record<string, unknown> = {}

	// Buffer console output during stepping to ensure correct order
	private isStepping = false
	private pendingConsoleOutput: Array<Record<string, unknown>> = []

	// Captured result from main() execution
	private scriptResult: unknown = undefined

	// Track if current pause is due to a breakpoint (for line number correction)
	private pausedAtBreakpoint = false

	// Environment variables to pass to the debugger subprocess (e.g., WM_WORKSPACE, WM_TOKEN, etc.)
	private envVars: Record<string, string> = {}

	// Nsjail configuration for sandboxed execution
	private nsjailConfig?: NsjailConfig

	// Custom bun binary path (can be overridden)
	private bunPath: string = 'bun'

	// Windmill binary path for prepare-deps CLI (optional, for dependency installation)
	private windmillPath?: string

	// Path to installed node_modules (set after prepare-deps runs)
	private nodeModulesPath?: string

	constructor(ws: WebSocket, options?: { nsjailConfig?: NsjailConfig; bunPath?: string; windmillPath?: string }) {
		this.ws = ws
		this.nsjailConfig = options?.nsjailConfig
		this.bunPath = options?.bunPath || 'bun'
		this.windmillPath = options?.windmillPath
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
	 * Handle script parsed event - track script IDs and parse source maps.
	 */
	private handleScriptParsed(script: V8Script): void {
		this.scripts.set(script.scriptId, script)

		// Identify the main script
		if (script.url && this.scriptPath && script.url.endsWith(this.scriptPath.split('/').pop()!)) {
			this.mainScriptId = script.scriptId
			logger.info(`Main script parsed: ${script.url} (ID: ${script.scriptId})`)

			// Parse source map to get bidirectional line mappings
			// This is crucial because Bun transpiles TypeScript and may strip blank lines/comments
			if (script.sourceMapURL) {
				logger.info('Parsing source map for line number mapping...')
				this.sourceMapMappings = parseSourceMapLineMapping(script.sourceMapURL)
				logger.info(`Source map parsed: ${this.sourceMapMappings.originalToTranspiled.size} original->transpiled, ${this.sourceMapMappings.transpiledToOriginal.size} transpiled->original`)
			}
		}
	}

	/**
	 * Apply breakpoints using URL pattern (works before script is parsed).
	 * This is called before the script starts running.
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
					// For file URLs, we match the end of the path
					const urlRegex = this.scriptPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

					// Line number conversion:
					// - User line N (1-indexed from DAP/Monaco)
					// - After debugger injection at line 0: user line N -> temp file line N (0-indexed)
					// - But Bun transpiles TypeScript and may strip blank lines/comments!
					// - We need to use the source map to find the correct transpiled line
					//
					// The source map maps original 0-indexed lines to transpiled 0-indexed lines.
					// Original line 0 is the injected debugger statement.
					// User line N (1-indexed) in original = line N (0-indexed) in temp file after injection.
					const originalLine0Indexed = line  // After injection, user line N = temp file line N (0-indexed)

					// Look up the transpiled line from the source map
					let transpiledLine = originalLine0Indexed
					if (this.sourceMapMappings.originalToTranspiled.size > 0) {
						const mappedLine = this.sourceMapMappings.originalToTranspiled.get(originalLine0Indexed)
						if (mappedLine !== undefined) {
							transpiledLine = mappedLine
							logger.info(`Source map: original line ${originalLine0Indexed} -> transpiled line ${transpiledLine}`)
						} else {
							// If not found in source map, try to find the nearest mapped line
							// This handles cases where the exact line isn't in the map (comments, blank lines)
							logger.warn(`Line ${originalLine0Indexed} not in source map, using as-is`)
						}
					}

					logger.info(`Setting breakpoint: user line ${line} -> original line ${originalLine0Indexed} (0-idx) -> transpiled line ${transpiledLine}`)
					const response = await this.sendInspectorCommand('Debugger.setBreakpointByUrl', {
						lineNumber: transpiledLine,
						urlRegex,
						columnNumber: 0
					})
					logger.info(`Breakpoint response: ${JSON.stringify(response)}`)
					if (response.result?.breakpointId) {
						ids.push(response.result.breakpointId as string)
						logger.info(`Breakpoint set at transpiled line ${transpiledLine} (user line ${line}, ID: ${response.result.breakpointId})`)
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

	// Track if this is the initial pause from our injected debugger statement
	private initialPauseDone = false

	/**
	 * Handle paused event.
	 */
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
			logger.info('Initial pause from injected debugger statement')

			// Re-apply breakpoints now that the script is fully parsed
			// This ensures breakpoints resolve to actual code locations
			logger.info('Re-applying breakpoints after script is parsed...')
			await this.applyBreakpointsByUrl()

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
		// lineNumber is 0-indexed in WebKit inspector (transpiled line)
		// We need to convert it back to the original user line using the source map
		let line: number | undefined
		if (callFrames && callFrames.length > 0) {
			const transpiledLine = callFrames[0].location.lineNumber
			// Convert transpiled line back to original line using source map
			if (this.sourceMapMappings.transpiledToOriginal.size > 0) {
				const originalLine = this.sourceMapMappings.transpiledToOriginal.get(transpiledLine)
				if (originalLine !== undefined) {
					line = originalLine
					logger.info(`Stopped: transpiled line ${transpiledLine} -> original line ${line}`)
				} else {
					// If not found in reverse map, use transpiled line as-is
					line = transpiledLine
					logger.warn(`Transpiled line ${transpiledLine} not in reverse source map, using as-is`)
				}
			} else {
				line = transpiledLine
			}
		}

		this.sendEvent('stopped', {
			reason: dapReason,
			threadId: 1,
			allThreadsStopped: true,
			line
		})

		// Flush any buffered console output that occurred during stepping
		this.isStepping = false
		for (const outputEvent of this.pendingConsoleOutput) {
			this.sendEvent('output', outputEvent)
		}
		this.pendingConsoleOutput = []
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
	 * Format a console parameter for output.
	 * Handles primitives, objects with preview, arrays, etc.
	 */
	private formatConsoleParameter(p: {
		type: string
		value?: unknown
		description?: string
		subtype?: string
		className?: string
		objectId?: string
		preview?: {
			type: string
			subtype?: string
			description?: string
			overflow?: boolean
			properties?: Array<{ name: string; type: string; value?: string; subtype?: string }>
		}
	}): string {
		// Handle primitives with direct values
		if (p.value !== undefined) {
			if (typeof p.value === 'string') return p.value
			return String(p.value)
		}

		// Handle null
		if (p.subtype === 'null') return 'null'

		// Handle undefined
		if (p.type === 'undefined') return 'undefined'

		// Handle objects with preview (this is the key fix for process.env, etc.)
		if (p.type === 'object' && p.preview && p.preview.properties) {
			const props = p.preview.properties
			const isArray = p.subtype === 'array' || p.preview.subtype === 'array'

			if (isArray) {
				// Format as array: [val1, val2, ...]
				const items = props.map((prop) => this.formatPreviewValue(prop))
				const suffix = p.preview.overflow ? ', ...' : ''
				return `[${items.join(', ')}${suffix}]`
			} else {
				// Format as object: { key1: val1, key2: val2, ... }
				const items = props.map((prop) => `${prop.name}: ${this.formatPreviewValue(prop)}`)
				const suffix = p.preview.overflow ? ', ...' : ''
				return `{ ${items.join(', ')}${suffix} }`
			}
		}

		// Handle functions
		if (p.type === 'function') {
			return p.description || '[Function]'
		}

		// Fallback to description (for other object types without preview)
		if (p.description) return p.description

		return ''
	}

	/**
	 * Format a single property from an object preview.
	 */
	private formatPreviewValue(prop: { name: string; type: string; value?: string; subtype?: string }): string {
		if (prop.subtype === 'null') return 'null'
		if (prop.type === 'undefined') return 'undefined'
		if (prop.type === 'string') return `"${prop.value ?? ''}"`
		if (prop.type === 'number' || prop.type === 'boolean') return prop.value ?? ''
		if (prop.type === 'function') return '[Function]'
		if (prop.type === 'object') {
			// Nested objects just show their type/value preview
			if (prop.subtype === 'array') return prop.value || '[]'
			return prop.value || '{...}'
		}
		return prop.value ?? ''
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
			line?: number
			column?: number
			url?: string
			parameters?: Array<{
				type: string
				value?: unknown
				description?: string
				subtype?: string
				className?: string
				objectId?: string
				preview?: {
					type: string
					subtype?: string
					description?: string
					overflow?: boolean
					properties?: Array<{ name: string; type: string; value?: string; subtype?: string }>
				}
			}>
		}

		if (!message) return

		let output: string
		if (message.parameters && message.parameters.length > 0) {
			output = message.parameters
				.map((p) => this.formatConsoleParameter(p))
				.join(' ')
		} else {
			output = message.text || ''
		}

		// Check for __WINDMILL_RESULT__ prefix and capture the result
		if (output.startsWith('__WINDMILL_RESULT__:')) {
			try {
				const resultJson = output.substring('__WINDMILL_RESULT__:'.length)
				this.scriptResult = JSON.parse(resultJson)
				logger.info(`Captured script result: ${resultJson}`)

				// Flush any pending console output before sending terminated
				// (output may have been buffered during stepping/continue)
				this.isStepping = false
				for (const pendingOutput of this.pendingConsoleOutput) {
					this.sendEvent('output', pendingOutput)
				}
				this.pendingConsoleOutput = []

				// Send terminated event immediately after capturing result
				// This is more reliable than waiting for process.exited or inspector.onclose
				if (!this.terminatedSent) {
					this.terminatedSent = true
					this.running = false
					logger.info('Sending terminated event after result capture')
					this.sendEvent('terminated', { result: this.scriptResult })
				}
			} catch (error) {
				logger.error('Failed to parse script result:', error)
			}
			// Don't send this as output to the client
			return
		}

		const category = message.level === 'error' || message.level === 'warning' ? 'stderr' : 'stdout'

		// Build the output event with optional source location
		const outputEvent: Record<string, unknown> = {
			category,
			output: output + '\n'
		}

		// If line info is available, include source reference
		// Line numbers from WebKit are 0-indexed, but we have injected debugger at line 0
		// So inspector line N = user line N (same as breakpoint handling)
		if (message.line !== undefined && message.url) {
			outputEvent.source = {
				path: message.url,
				name: message.url.split('/').pop() || 'script.ts'
			}
			outputEvent.line = message.line // 0-indexed + 1 - 1 for injected debugger = same
			if (message.column !== undefined) {
				outputEvent.column = message.column + 1
			}
		}

		// Buffer output during stepping to ensure it appears after the stopped event
		if (this.isStepping) {
			this.pendingConsoleOutput.push(outputEvent)
		} else {
			this.sendEvent('output', outputEvent)
		}
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
			logger.info(`Launch with env vars: ${Object.keys(this.envVars).join(', ')}`)
		}

		if (!this.scriptPath && !code) {
			this.sendResponse(request, false, {}, 'No program or code specified')
			return
		}

		// Prepare dependencies using the original code (before any modifications)
		// This analyzes imports and installs required npm packages
		if (code) {
			this.nodeModulesPath = await this.prepareDependencies(code) || undefined

			// Remove version specifiers from imports (e.g., "lodash@4" -> "lodash")
			// This must happen AFTER prepareDependencies (which needs the versions)
			// but BEFORE the code is executed (Bun doesn't understand @version syntax)
			code = removePinnedImports(code)
		}

		// Reset state for new launch
		this.initialPauseDone = false

		// Inject a debugger statement at the beginning to force initial pause
		// This gives us a known pause point where we can ensure breakpoints are set
		code = `debugger; // Auto-injected by Windmill debugger\n${code || ''}`

		// If callMain is true, append a call to main() with the provided args
		if (this.callMain && code) {
			// Generate positional arguments list (like Python's keyword args)
			// For TypeScript, we pass arguments in order
			const argsValues = Object.values(this.mainArgs)
				.map((v) => JSON.stringify(v))
				.join(', ')
			code =
				code +
				`\n\n// Auto-generated call to main entrypoint\n` +
				`globalThis.__windmill_result__ = await main(${argsValues});\n` +
				`console.log("__WINDMILL_RESULT__:" + JSON.stringify(globalThis.__windmill_result__));\n` +
				`// Small delay to ensure console output is delivered via inspector before process exits\n` +
				`await new Promise(r => setTimeout(r, 50));\n`
			logger.info(`Added main() call with args: ${argsValues}`)
		}

		// Write code to temp file if provided
		if (code && !this.scriptPath) {
			try {
				this.tempDir = await mkdtemp(join(tmpdir(), 'windmill_debug_'))
				this.tempFile = join(this.tempDir, 'script.ts')
				await writeFile(this.tempFile, code)
				this.scriptPath = this.tempFile
				logger.info(`Wrote code to ${this.tempFile}`)
				// Log lines around breakpoint for debugging
				const lines = code.split('\n')
				for (let i = 24; i < Math.min(30, lines.length); i++) {
					logger.info(`  Line ${i} (0-idx) / ${i+1} (1-idx): ${lines[i]?.substring(0, 80)}`)
				}

				// If we have installed node_modules, symlink them into the temp directory
				// so Bun can find them when running the script
				if (this.nodeModulesPath) {
					const targetPath = join(this.tempDir, 'node_modules')
					try {
						await symlink(this.nodeModulesPath, targetPath)
						logger.info(`Symlinked ${this.nodeModulesPath} -> ${targetPath}`)
					} catch (symlinkError) {
						logger.warn(`Failed to symlink node_modules: ${symlinkError}`)
						// Don't fail the launch - try NODE_PATH as fallback
					}
				}
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
	 * Prepare dependencies by calling the windmill CLI's prepare-deps command.
	 * This analyzes imports in the code and installs required npm packages.
	 * Returns the path to node_modules if any were installed.
	 */
	private async prepareDependencies(code: string, language: string = 'bun'): Promise<string | null> {
		if (!this.windmillPath) {
			logger.info('No windmill binary path configured, skipping dependency preparation')
			return null
		}

		logger.info(`Preparing dependencies using ${this.windmillPath}`)

		try {
			const input = JSON.stringify({ code, language }) + '\n'
			logger.info(`prepare-deps input length: ${input.length}`)

			// Spawn the windmill binary with prepare-deps command
			const proc = spawn({
				cmd: [this.windmillPath, 'prepare-deps'],
				stdin: new Blob([input]),  // Use Blob for complete stdin data
				stdout: 'pipe',
				stderr: 'pipe'
			})

			// Wait for completion
			const output = await new Response(proc.stdout).text()
			const stderr = await new Response(proc.stderr).text()
			logger.info(`prepare-deps output: ${output.substring(0, 200)}`)
			logger.info(`prepare-deps stderr: ${stderr.substring(0, 200)}`)

			if (stderr) {
				// Filter out the "Running in standalone mode" message
				const filteredStderr = stderr.split('\n').filter(line => !line.includes('Running in standalone mode')).join('\n').trim()
				if (filteredStderr) {
					logger.debug(`prepare-deps stderr: ${filteredStderr}`)
				}
			}

			// Parse the JSON response
			const response = JSON.parse(output.trim().split('\n').pop() || '{}')

			if (!response.success) {
				logger.error(`prepare-deps failed: ${response.error}`)
				this.sendEvent('output', {
					category: 'console',
					output: `Warning: Failed to prepare dependencies: ${response.error}\n`
				})
				return null
			}

			if (response.node_modules_path) {
				logger.info(`Dependencies installed at: ${response.node_modules_path}`)
				this.sendEvent('output', {
					category: 'console',
					output: `Dependencies installed at: ${response.node_modules_path}\n`
				})
				return response.node_modules_path
			}

			logger.info('No external dependencies to install')
			return null
		} catch (error) {
			logger.error(`Failed to prepare dependencies: ${error}`)
			this.sendEvent('output', {
				category: 'console',
				output: `Warning: Failed to prepare dependencies: ${error}\n`
			})
			return null
		}
	}

	/**
	 * Start the Bun subprocess with debugging enabled.
	 */
	private async startBunProcess(cwd: string): Promise<void> {
		if (!this.scriptPath) {
			throw new Error('No script path')
		}

		const inspectPort = 9229 + Math.floor(Math.random() * 1000)
		const inspectUrl = `127.0.0.1:${inspectPort}`

		// Build the command - optionally wrapped with nsjail
		let cmd: string[] = [this.bunPath, `--inspect-wait=${inspectUrl}`, this.scriptPath]

		if (this.nsjailConfig?.enabled) {
			const nsjailCmd = [this.nsjailConfig.binaryPath]

			if (this.nsjailConfig.configPath) {
				nsjailCmd.push('--config', this.nsjailConfig.configPath)
			}

			if (this.nsjailConfig.extraArgs) {
				nsjailCmd.push(...this.nsjailConfig.extraArgs)
			}

			nsjailCmd.push('--cwd', cwd)
			nsjailCmd.push('--')
			nsjailCmd.push(...cmd)

			cmd = nsjailCmd
			logger.info(`Starting Bun with nsjail: ${cmd.join(' ')}`)
		} else {
			logger.info(`Starting Bun with --inspect-wait=${inspectUrl}`)
		}

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

		// Only include essential env vars + client-provided ones
		// Don't inherit all of process.env to keep debugger environment clean
		const envVars: Record<string, string | undefined> = {
			// Essential system vars
			PATH: process.env.PATH || '/usr/bin:/bin',
			HOME: process.env.HOME,
			// Client-provided env vars (WM_WORKSPACE, WM_TOKEN, etc.)
			// Note: WM_BASE_URL is already overridden by BASE_INTERNAL_URL if set
			...this.envVars
		}

		// If we have installed node_modules, add NODE_PATH so Bun can find them
		if (this.nodeModulesPath) {
			envVars.NODE_PATH = this.nodeModulesPath
			logger.info(`Setting NODE_PATH=${this.nodeModulesPath}`)
		}

		this.process = spawn({
			cmd,
			cwd,
			stdout: 'pipe',
			stderr: 'pipe',
			env: envVars
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
		this.process.exited.then(async (exitCode) => {
			logger.info(`Bun process exited with code: ${exitCode}`)
			this.running = false

			// Send terminated event BEFORE cleanup to ensure WebSocket is still open
			if (!this.terminatedSent) {
				this.terminatedSent = true
				logger.info(`Sending terminated event with result: ${JSON.stringify(this.scriptResult)}`)
				try {
					this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
					logger.info('Terminated event sent successfully')
				} catch (error) {
					logger.error('Failed to send terminated event:', error)
				}
			}

			await this.cleanup()
		}).catch((error) => {
			logger.error('Error in process.exited handler:', error)
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

				// When inspector closes, the script has ended - send terminated event
				if (!this.terminatedSent) {
					this.terminatedSent = true
					this.running = false
					logger.info(`Inspector closed - sending terminated event with result: ${JSON.stringify(this.scriptResult)}`)
					try {
						this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
						logger.info('Terminated event sent successfully via inspector close')
					} catch (error) {
						logger.error('Failed to send terminated event on inspector close:', error)
					}
				}
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
		logger.info(`handleStackTrace: callFrames.length=${this.callFrames.length}`)
		const stackFrames: StackFrame[] = []

		// Filter to only include frames from the user's script
		// The call stack should start at module code, not include internal Bun/loader frames
		for (let i = 0; i < this.callFrames.length; i++) {
			const frame = this.callFrames[i]
			const script = this.scripts.get(frame.location.scriptId)

			logger.info(`  frame ${i}: functionName="${frame.functionName}", scriptId=${frame.location.scriptId}, lineNumber=${frame.location.lineNumber}, scriptUrl=${script?.url}`)

			// Only include frames from the main script
			if (!script?.url || !this.scriptPath) {
				logger.info(`  skipping frame ${i}: no script URL or scriptPath`)
				continue
			}

			// Check if this frame is from the user's script
			const isUserScript = script.url.endsWith(this.scriptPath.split('/').pop()!)
			if (!isUserScript) {
				logger.info(`  skipping frame ${i}: not from user script (${script.url})`)
				continue
			}

			// Inspector line is 0-indexed (transpiled line)
			// We need to convert it back to the original user line using the source map
			const transpiledLine = frame.location.lineNumber
			let frameLine = transpiledLine
			if (this.sourceMapMappings.transpiledToOriginal.size > 0) {
				const originalLine = this.sourceMapMappings.transpiledToOriginal.get(transpiledLine)
				if (originalLine !== undefined) {
					frameLine = originalLine
				}
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

		logger.info(`handleScopes: frameId=${frameId}, frameIndex=${frameIndex}, callFrames.length=${this.callFrames.length}`)

		const frame = this.callFrames[frameIndex]
		if (!frame) {
			logger.warn(`handleScopes: No frame at index ${frameIndex}`)
			this.sendResponse(request, true, { scopes: [] })
			return
		}

		logger.info(`handleScopes: frame.scopeChain has ${frame.scopeChain.length} scopes`)
		for (const s of frame.scopeChain) {
			logger.info(`  scope: type=${s.type}, name=${s.name}, hasObjectId=${!!s.object.objectId}`)
		}

		const scopes: Array<{ name: string; variablesReference: number; expensive: boolean }> = []

		for (const scope of frame.scopeChain) {
			const ref = this.nextVarRef()
			const objectId = scope.object.objectId

			if (objectId) {
				this.scopesMap.set(ref, { type: scope.type, objectId, frameIndex })

				// Format the name nicely
				let name: string
				if (scope.name) {
					name = scope.name
				} else {
					name = scope.type.charAt(0).toUpperCase() + scope.type.slice(1)
				}

				scopes.push({
					name,
					variablesReference: ref,
					expensive: scope.type === 'global'
				})
				logger.info(`  Added scope: name=${name}, ref=${ref}, type=${scope.type}`)
			} else {
				logger.warn(`  Scope ${scope.type} has no objectId, skipping`)
			}
		}

		logger.info(`handleScopes: returning ${scopes.length} scopes`)
		this.sendResponse(request, true, { scopes })
	}

	/**
	 * Handle the 'variables' request.
	 */
	async handleVariables(request: DAPMessage): Promise<void> {
		const args = request.arguments || {}
		const variablesRef = (args.variablesReference as number) || 0

		logger.info(`handleVariables: variablesRef=${variablesRef}`)

		const scopeInfo = this.scopesMap.get(variablesRef)
		const objectId = this.objectsMap.get(variablesRef) || scopeInfo?.objectId

		logger.info(`handleVariables: scopeInfo=${JSON.stringify(scopeInfo)}, objectId=${objectId}`)

		if (!objectId) {
			logger.warn(`handleVariables: No objectId found for ref ${variablesRef}`)
			this.sendResponse(request, true, { variables: [] })
			return
		}

		try {
			// Determine scope type for filtering
			const scopeType = scopeInfo?.type || 'unknown'
			const isGlobalScope = scopeType === 'global'

			logger.info(`handleVariables: calling Runtime.getProperties for objectId=${objectId}, scopeType=${scopeType}`)

			const response = await this.sendInspectorCommand('Runtime.getProperties', {
				objectId,
				ownProperties: true,
				generatePreview: true
			})

			// Log all property names for debugging
			const allProps = (response.result?.properties as Array<{ name: string }>) || []
			logger.info(`handleVariables: all property names: ${allProps.map(p => p.name).join(', ')}`)

			logger.info(`handleVariables: got response with ${(response.result?.properties as unknown[])?.length || 0} properties for scope type: ${scopeType}`)

			const variables: Variable[] = []
			const properties = (response.result?.properties as Array<{
				name: string
				value?: {
					type: string
					value?: unknown
					description?: string
					objectId?: string
					subtype?: string
					className?: string
					preview?: {
						type: string
						subtype?: string
						description?: string
						overflow?: boolean
						properties?: Array<{ name: string; type: string; value?: string; subtype?: string }>
					}
				}
				configurable?: boolean
				enumerable?: boolean
				writable?: boolean
			}>) || []

			for (const prop of properties) {
				if (!prop.value) continue

				// Skip internal properties (start with __)
				if (prop.name.startsWith('__') && prop.name !== '__windmill_result__') continue

				// Skip native functions (built-in methods)
				if (prop.value.type === 'function' && prop.value.description?.includes('[native code]')) continue

				// Skip common built-in object names
				const builtInNames = new Set([
					'NaN', 'Infinity', 'undefined', 'globalThis', 'global', 'self', 'window',
					'console', 'Bun', 'process', 'navigator', 'performance', 'crypto', 'Loader',
					'onmessage', 'onerror', 'toString', 'toLocaleString', 'valueOf',
					'hasOwnProperty', 'propertyIsEnumerable', 'isPrototypeOf', 'constructor',
					// Built-in objects
					'Reflect', 'JSON', 'Math', 'Atomics', 'Intl', 'WebAssembly', 'Proxy',
					'Object', 'Array', 'Function', 'Boolean', 'Symbol', 'Number', 'BigInt',
					'String', 'RegExp', 'Date', 'Promise', 'Map', 'Set', 'WeakMap', 'WeakSet',
					'Error', 'TypeError', 'RangeError', 'SyntaxError', 'ReferenceError',
					'EvalError', 'URIError', 'AggregateError', 'ArrayBuffer', 'DataView',
					'Int8Array', 'Uint8Array', 'Uint8ClampedArray', 'Int16Array', 'Uint16Array',
					'Int32Array', 'Uint32Array', 'Float32Array', 'Float64Array', 'BigInt64Array',
					'BigUint64Array', 'SharedArrayBuffer'
				])
				if (builtInNames.has(prop.name)) continue

				// Skip properties that look like class/constructor names (PascalCase and are functions)
				if (prop.value.type === 'function' && /^[A-Z][a-zA-Z0-9]*$/.test(prop.name)) continue

				let value: string
				let varRef = 0
				let displayType = prop.value.type
				const className = prop.value.className

				// Log for debugging variable parsing issues
				logger.debug(`Variable ${prop.name}: type=${prop.value.type}, className=${className}, description=${prop.value.description}, value=${JSON.stringify(prop.value.value)}`)

				if (prop.value.type === 'object' && prop.value.objectId) {
					// Check if this is a boxed primitive (String, Number, Boolean)
					if (className === 'String' && prop.value.description) {
						// Boxed string - description contains the string value
						value = prop.value.description.startsWith('"') ? prop.value.description : `"${prop.value.description}"`
						displayType = 'string'
					} else if (className === 'Number' && prop.value.description) {
						value = prop.value.description
						displayType = 'number'
					} else if (className === 'Boolean' && prop.value.description) {
						value = prop.value.description
						displayType = 'boolean'
					} else {
						// Regular object - create a reference for nested inspection
						varRef = this.nextVarRef()
						this.objectsMap.set(varRef, prop.value.objectId)
						// Use preview data to show object contents instead of just "Object"
						if (prop.value.preview && prop.value.preview.properties) {
							const previewProps = prop.value.preview.properties
							const isArray = prop.value.subtype === 'array' || prop.value.preview.subtype === 'array'
							if (isArray) {
								const items = previewProps.map((p) => this.formatPreviewValue(p))
								const suffix = prop.value.preview.overflow ? ', ...' : ''
								value = `[${items.join(', ')}${suffix}]`
							} else {
								const items = previewProps.map((p) => `${p.name}: ${this.formatPreviewValue(p)}`)
								const suffix = prop.value.preview.overflow ? ', ...' : ''
								value = `{ ${items.join(', ')}${suffix} }`
							}
						} else {
							value = prop.value.description || `[${prop.value.subtype || className || 'Object'}]`
						}
					}
				} else if (prop.value.type === 'string') {
					// Handle primitive strings - they have value property
					value = prop.value.value !== undefined ? JSON.stringify(prop.value.value) : (prop.value.description || '""')
					displayType = 'string'
				} else if (prop.value.value !== undefined) {
					value = JSON.stringify(prop.value.value)
				} else {
					value = prop.value.description || String(prop.value.type)
				}

				variables.push({
					name: prop.name,
					value,
					type: displayType,
					variablesReference: varRef
				})
			}

			logger.info(`handleVariables: returning ${variables.length} variables (filtered from ${properties.length})`)
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
		let expression = (args.expression as string) || ''
		const frameId = args.frameId as number | undefined
		const token = args.token as string | undefined

		// Verify expression token if provided (optional - for audit logging)
		// Expression tokens are signed by the backend to create audit logs
		if (token) {
			const verificationError = await verifyExpressionToken(token, expression)
			if (verificationError) {
				// Log the error but don't block evaluation
				// Expression signing is for audit logging, not security enforcement
				logger.warn(`Expression token verification failed: ${verificationError}`)
			}
		}

		// If expression starts with 'await ', strip it and we'll await the promise result
		// This preserves the scope context unlike wrapping in an async IIFE
		const shouldAwait = /^\s*await\s+/.test(expression)
		if (shouldAwait) {
			expression = expression.replace(/^\s*await\s+/, '')
		}

		try {
			let result: string
			let varRef = 0

			// Helper to format evaluation result
			const formatResult = async (evalResult: {
				type: string
				subtype?: string
				className?: string
				value?: unknown
				description?: string
				objectId?: string
				preview?: { properties?: Array<{ name: string; value: string }> }
			} | undefined): Promise<{ result: string; varRef: number }> => {
				let result = 'undefined'
				let varRef = 0

				if (!evalResult) return { result, varRef }

				// Check if result is a Promise - extract value from preview if already settled
				// (Runtime.awaitPromise doesn't work reliably in Bun's inspector)
				if (evalResult.subtype === 'promise' || evalResult.className === 'Promise') {
					if (evalResult.preview?.properties) {
						const props = evalResult.preview.properties as Array<{
							name: string
							type?: string
							value?: string
							subtype?: string
							valuePreview?: { properties?: Array<{ name: string; value?: string }> }
						}>
						const statusProp = props.find(p => p.name === 'status')
						const resultProp = props.find(p => p.name === 'result')

						if (statusProp?.value === 'fulfilled' && resultProp) {
							// Promise is already fulfilled - return the result directly
							// For primitives, value is directly available
							if (resultProp.value !== undefined) {
								return { result: resultProp.value, varRef: 0 }
							}
							// For objects, format from valuePreview or show type
							if (resultProp.type === 'object') {
								if (resultProp.valuePreview?.properties) {
									const objProps = resultProp.valuePreview.properties
										.map(p => `${p.name}: ${p.value ?? '...'}`)
										.join(', ')
									return { result: `{${objProps}}`, varRef: 0 }
								}
								// Fallback: show the subtype or generic object
								return { result: resultProp.subtype || '[Object]', varRef: 0 }
							}
							// For other types (function, symbol, etc)
							return { result: resultProp.type || 'undefined', varRef: 0 }
						} else if (statusProp?.value === 'rejected' && resultProp) {
							// Promise was rejected - show the error
							return { result: `Rejected: ${resultProp.value ?? resultProp.type}`, varRef: 0 }
						}
						// For pending promises, show that it's pending
						if (statusProp?.value === 'pending') {
							return { result: 'Promise { <pending> }', varRef: 0 }
						}
					}
				}

				if (evalResult.type === 'undefined') {
					result = 'undefined'
				} else if (evalResult.value !== undefined) {
					result = JSON.stringify(evalResult.value)
				} else if (evalResult.objectId) {
					varRef = this.nextVarRef()
					this.objectsMap.set(varRef, evalResult.objectId)
					// Use preview if available for better display
					if (evalResult.preview?.properties) {
						const props = evalResult.preview.properties.map(p => `${p.name}: ${p.value}`).join(', ')
						result = `{${props}}`
					} else {
						result = evalResult.description || '[Object]'
					}
				} else {
					result = evalResult.description || String(evalResult.type)
				}

				return { result, varRef }
			}

			if (frameId !== undefined && this.callFrames[frameId - 1]) {
				const frame = this.callFrames[frameId - 1]
				const response = await this.sendInspectorCommand('Debugger.evaluateOnCallFrame', {
					callFrameId: frame.callFrameId,
					expression,
					returnByValue: false,
					generatePreview: true
				})

				const evalResult = response.result?.result as {
					type: string
					subtype?: string
					className?: string
					value?: unknown
					description?: string
					objectId?: string
					preview?: { properties?: Array<{ name: string; value: string }> }
				}

				const formatted = await formatResult(evalResult)
				result = formatted.result
				varRef = formatted.varRef
			} else {
				const response = await this.sendInspectorCommand('Runtime.evaluate', {
					expression,
					returnByValue: false,
					generatePreview: true
				})

				const evalResult = response.result?.result as {
					type: string
					subtype?: string
					className?: string
					value?: unknown
					description?: string
					objectId?: string
					preview?: { properties?: Array<{ name: string; value: string }> }
				}

				const formatted = await formatResult(evalResult)
				result = formatted.result
				varRef = formatted.varRef
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
			this.isStepping = true
			this.pausedAtBreakpoint = false // Reset for next pause
			await this.sendInspectorCommand('Debugger.resume', {})
			this.sendResponse(request, true, { allThreadsContinued: true })
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'next' (step over) request.
	 */
	async handleNext(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false // Reset for next pause
			await this.sendInspectorCommand('Debugger.stepOver', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'stepIn' request.
	 */
	async handleStepIn(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false // Reset for next pause
			await this.sendInspectorCommand('Debugger.stepInto', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
			this.sendResponse(request, false, {}, String(error))
		}
	}

	/**
	 * Handle the 'stepOut' request.
	 */
	async handleStepOut(request: DAPMessage): Promise<void> {
		try {
			this.isStepping = true
			this.pausedAtBreakpoint = false // Reset for next pause
			await this.sendInspectorCommand('Debugger.stepOut', {})
			this.sendResponse(request)
		} catch (error) {
			this.isStepping = false
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
		// Set flag BEFORE cleanup to prevent inspector onclose from sending duplicate
		const shouldSendTerminated = !this.terminatedSent
		this.terminatedSent = true
		await this.cleanup()
		this.sendResponse(request)
		// Include the captured result if available
		if (shouldSendTerminated) {
			this.sendEvent('terminated', this.scriptResult !== undefined ? { result: this.scriptResult } : {})
		}
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
let windmillPath: string | undefined

for (let i = 0; i < args.length; i++) {
	if (args[i] === '--host' && args[i + 1]) {
		host = args[i + 1]
		i++
	} else if (args[i] === '--port' && args[i + 1]) {
		port = parseInt(args[i + 1], 10)
		i++
	} else if (args[i] === '--windmill' && args[i + 1]) {
		windmillPath = args[i + 1]
		i++
	}
}

// Only start the standalone server when run directly (not when imported)
if (import.meta.main) {
	if (windmillPath) {
		logger.info(`Windmill binary path: ${windmillPath}`)
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

				const session = new DebugSession(wsWrapper, { windmillPath })
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
}
