/**
 * DAP (Debug Adapter Protocol) WebSocket Client for Monaco integration.
 *
 * This client implements the DAP protocol over WebSocket to communicate
 * with a Python debugpy-based debug server.
 */

import { writable, get } from 'svelte/store'

export interface DAPMessage {
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

export interface Breakpoint {
	id: number
	verified: boolean
	line: number
	source?: { path: string; name?: string }
}

export interface StackFrame {
	id: number
	name: string
	source?: { path: string; name?: string }
	line: number
	column: number
}

export interface Variable {
	name: string
	value: string
	type?: string
	variablesReference: number
}

export interface Scope {
	name: string
	variablesReference: number
	expensive: boolean
}

export interface DebugState {
	connected: boolean
	initialized: boolean
	running: boolean
	stopped: boolean
	stoppedReason?: string
	currentLine?: number
	currentFile?: string
	stackFrames: StackFrame[]
	scopes: Scope[]
	variables: Map<number, Variable[]>
	breakpoints: Map<string, Breakpoint[]>
	output: string[]
	logs: string
	result?: unknown
	error?: string
}

const initialState: DebugState = {
	connected: false,
	initialized: false,
	running: false,
	stopped: false,
	stackFrames: [],
	scopes: [],
	variables: new Map(),
	breakpoints: new Map(),
	output: [],
	logs: '',
	result: undefined,
	error: undefined
}

export const debugState = writable<DebugState>({ ...initialState })

export class DAPClient {
	private ws: WebSocket | null = null
	private seq = 1
	private pendingRequests: Map<number, { resolve: (value: DAPMessage) => void; reject: (error: Error) => void }> = new Map()
	private url: string

	constructor(url: string = 'ws://localhost:5679') {
		this.url = url
	}

	/**
	 * Connect to the DAP server.
	 */
	async connect(): Promise<void> {
		return new Promise((resolve, reject) => {
			try {
				this.ws = new WebSocket(this.url)

				this.ws.onopen = () => {
					debugState.update((s) => ({ ...s, connected: true }))
					resolve()
				}

				this.ws.onclose = () => {
					debugState.update((s) => ({
						...initialState,
						breakpoints: s.breakpoints
					}))
					this.pendingRequests.clear()
				}

				this.ws.onerror = (error) => {
					console.error('DAP WebSocket error:', error)
					reject(new Error('WebSocket connection failed'))
				}

				this.ws.onmessage = (event) => {
					this.handleMessage(event.data)
				}
			} catch (error) {
				reject(error)
			}
		})
	}

	/**
	 * Disconnect from the DAP server.
	 */
	disconnect(): void {
		if (this.ws) {
			this.ws.close()
			this.ws = null
		}
		debugState.set({ ...initialState })
	}

	/**
	 * Send a request to the DAP server and wait for a response.
	 */
	private async sendRequest(command: string, args?: Record<string, unknown>): Promise<DAPMessage> {
		if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
			throw new Error('Not connected to DAP server')
		}

		const seq = this.seq++
		const message: DAPMessage = {
			seq,
			type: 'request',
			command,
			arguments: args
		}

		return new Promise((resolve, reject) => {
			this.pendingRequests.set(seq, { resolve, reject })

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

			this.ws!.send(JSON.stringify(message))
		})
	}

	/**
	 * Handle incoming messages from the DAP server.
	 */
	private handleMessage(data: string): void {
		try {
			const message: DAPMessage = JSON.parse(data)

			if (message.type === 'response') {
				const pending = this.pendingRequests.get(message.request_seq!)
				if (pending) {
					this.pendingRequests.delete(message.request_seq!)
					if (message.success) {
						pending.resolve(message)
					} else {
						pending.reject(new Error(message.message || 'Request failed'))
					}
				}
			} else if (message.type === 'event') {
				this.handleEvent(message)
			}
		} catch (error) {
			console.error('Failed to parse DAP message:', error)
		}
	}

	/**
	 * Handle DAP events.
	 */
	private handleEvent(event: DAPMessage): void {
		const body = event.body as Record<string, unknown> | undefined

		switch (event.event) {
			case 'initialized':
				// Clear logs and result when starting a new session
				debugState.update((s) => ({
					...s,
					initialized: true,
					logs: '',
					output: [],
					result: undefined,
					error: undefined
				}))
				break

			case 'stopped':
				debugState.update((s) => ({
					...s,
					stopped: true,
					running: false,
					stoppedReason: body?.reason as string,
					currentLine: body?.line as number | undefined
				}))
				// Automatically fetch stack trace when stopped
				this.fetchStackTrace()
				break

			case 'continued':
				debugState.update((s) => ({
					...s,
					stopped: false,
					running: true,
					stoppedReason: undefined
				}))
				break

			case 'terminated':
				debugState.update((s) => ({
					...s,
					running: false,
					stopped: false,
					initialized: false,
					result: body?.result,
					error: body?.error as string | undefined
				}))
				break

			case 'output':
				if (body?.output) {
					debugState.update((s) => ({
						...s,
						output: [...s.output, body.output as string],
						logs: s.logs + (body.output as string)
					}))
				}
				break

			case 'breakpoint':
				// Breakpoint was verified/modified by the server
				break

			default:
				console.log('Unhandled DAP event:', event.event)
		}
	}

	/**
	 * Initialize the debug session.
	 */
	async initialize(): Promise<DAPMessage> {
		const response = await this.sendRequest('initialize', {
			clientID: 'windmill',
			clientName: 'Windmill Script Editor',
			adapterID: 'python',
			pathFormat: 'path',
			linesStartAt1: true,
			columnsStartAt1: true,
			supportsVariableType: true,
			supportsVariablePaging: false,
			supportsRunInTerminalRequest: false,
			locale: 'en-US'
		})
		return response
	}

	/**
	 * Set breakpoints in a source file.
	 */
	async setBreakpoints(path: string, lines: number[]): Promise<Breakpoint[]> {
		const response = await this.sendRequest('setBreakpoints', {
			source: { path },
			breakpoints: lines.map((line) => ({ line }))
		})

		const breakpoints = (response.body?.breakpoints as Breakpoint[]) || []

		debugState.update((s) => {
			const newBreakpoints = new Map(s.breakpoints)
			newBreakpoints.set(path, breakpoints)
			return { ...s, breakpoints: newBreakpoints }
		})

		return breakpoints
	}

	/**
	 * Notify the server that configuration is done.
	 */
	async configurationDone(): Promise<void> {
		await this.sendRequest('configurationDone')
	}

	/**
	 * Launch a Python script for debugging.
	 */
	async launch(options: {
		program?: string
		code?: string
		args?: string[] | Record<string, unknown>
		cwd?: string
		callMain?: boolean
	}): Promise<void> {
		await this.sendRequest('launch', {
			program: options.program,
			code: options.code,
			args: options.args || {},
			cwd: options.cwd || '.',
			callMain: options.callMain || false
		})
		debugState.update((s) => ({ ...s, running: true }))
	}

	/**
	 * Continue execution.
	 */
	async continue_(): Promise<void> {
		await this.sendRequest('continue', { threadId: 1 })
		debugState.update((s) => ({ ...s, stopped: false, running: true }))
	}

	/**
	 * Step over (next).
	 */
	async stepOver(): Promise<void> {
		await this.sendRequest('next', { threadId: 1 })
	}

	/**
	 * Step into.
	 */
	async stepIn(): Promise<void> {
		await this.sendRequest('stepIn', { threadId: 1 })
	}

	/**
	 * Step out.
	 */
	async stepOut(): Promise<void> {
		await this.sendRequest('stepOut', { threadId: 1 })
	}

	/**
	 * Pause execution.
	 */
	async pause(): Promise<void> {
		await this.sendRequest('pause', { threadId: 1 })
	}

	/**
	 * Terminate the debug session.
	 */
	async terminate(): Promise<void> {
		await this.sendRequest('terminate')
	}

	/**
	 * Get threads.
	 */
	async getThreads(): Promise<{ id: number; name: string }[]> {
		const response = await this.sendRequest('threads')
		return (response.body?.threads as { id: number; name: string }[]) || []
	}

	/**
	 * Get stack trace.
	 */
	async getStackTrace(threadId: number = 1): Promise<StackFrame[]> {
		const response = await this.sendRequest('stackTrace', {
			threadId,
			startFrame: 0,
			levels: 20
		})
		const frames = (response.body?.stackFrames as StackFrame[]) || []

		debugState.update((s) => ({
			...s,
			stackFrames: frames,
			currentLine: frames[0]?.line,
			currentFile: frames[0]?.source?.path
		}))

		return frames
	}

	/**
	 * Get scopes for a stack frame.
	 */
	async getScopes(frameId: number): Promise<Scope[]> {
		const response = await this.sendRequest('scopes', { frameId })
		const scopes = (response.body?.scopes as Scope[]) || []

		debugState.update((s) => ({ ...s, scopes }))

		return scopes
	}

	/**
	 * Get variables for a scope.
	 */
	async getVariables(variablesReference: number): Promise<Variable[]> {
		const response = await this.sendRequest('variables', { variablesReference })
		const variables = (response.body?.variables as Variable[]) || []

		debugState.update((s) => {
			const newVariables = new Map(s.variables)
			newVariables.set(variablesReference, variables)
			return { ...s, variables: newVariables }
		})

		return variables
	}

	/**
	 * Evaluate an expression.
	 */
	async evaluate(
		expression: string,
		frameId?: number,
		context: 'watch' | 'repl' | 'hover' = 'repl'
	): Promise<{ result: string; variablesReference: number }> {
		const response = await this.sendRequest('evaluate', {
			expression,
			frameId,
			context
		})
		return {
			result: response.body?.result as string,
			variablesReference: response.body?.variablesReference as number
		}
	}

	/**
	 * Fetch stack trace (called automatically when stopped).
	 */
	private async fetchStackTrace(): Promise<void> {
		try {
			const frames = await this.getStackTrace()
			if (frames.length > 0) {
				await this.getScopes(frames[0].id)
			}
		} catch (error) {
			console.error('Failed to fetch stack trace:', error)
		}
	}

	/**
	 * Clear output.
	 */
	clearOutput(): void {
		debugState.update((s) => ({ ...s, output: [] }))
	}

	/**
	 * Check if connected.
	 */
	isConnected(): boolean {
		return get(debugState).connected
	}

	/**
	 * Check if running.
	 */
	isRunning(): boolean {
		return get(debugState).running
	}

	/**
	 * Check if stopped at a breakpoint.
	 */
	isStopped(): boolean {
		return get(debugState).stopped
	}
}

// Singleton instance
let dapClientInstance: DAPClient | null = null

export function getDAPClient(url?: string): DAPClient {
	if (!dapClientInstance) {
		dapClientInstance = new DAPClient(url)
	}
	return dapClientInstance
}

export function resetDAPClient(): void {
	if (dapClientInstance) {
		dapClientInstance.disconnect()
		dapClientInstance = null
	}
}
