import * as vscode from 'vscode'
import { editor as meditor, MarkerSeverity, Uri } from 'monaco-editor'
import { MonacoLanguageClient } from 'monaco-languageclient'
import { toSocket, WebSocketMessageReader, WebSocketMessageWriter } from 'vscode-ws-jsonrpc'
import { CloseAction, ErrorAction } from 'vscode-languageclient'
import type { ScriptLintResult } from '../copilot/chat/shared'
import { initializeVscode } from '../vscode'
import { computeModelPath, computeModelUri } from './monacoUri'
import { buildDenoImportMap, lspServersFor, type LspServerConfig } from './lspLanguageConfig'
import { genAtaRoot } from './typescriptAta'

// Lints LSP-backed languages (python, go, deno, shell) without an editor. Unlike the
// TypeScript worker, these servers are per-document: the editor gives every such model a
// randomized path, so there is no shared model to reuse. Parity comes from running the
// same servers with the same initialization options.
//
// Every step here is bounded. A server that accepts the socket but never answers
// `initialize` would otherwise leave the chat waiting forever.

const DEFAULT_TIMEOUT_MS = 15000
const STOP_TIMEOUT_MS = 2000
// How long a server that answered a diagnostic request with nothing is still given to
// publish instead. Ruff answers the request but reports through publishDiagnostics.
const PUSH_FALLBACK_MS = 2500

const silentOutputChannel = {
	name: 'Headless Lint Language Client',
	appendLine: () => {},
	append: () => {},
	clear: () => {},
	replace: () => {},
	show: () => {},
	hide: () => {},
	dispose: () => {}
}

interface StartedClient {
	server: LspServerConfig
	client: MonacoLanguageClient
	webSocket: WebSocket
	pushedDiagnostics: Promise<void>
}

function withDeadline<T>(promise: Promise<T>, ms: number, onTimeout: T): Promise<T> {
	return Promise.race([
		promise,
		new Promise<T>((resolve) => setTimeout(() => resolve(onTimeout), ms))
	])
}

function startClient(
	server: LspServerConfig,
	editorLang: string,
	uri: Uri,
	deadlineMs: number
): Promise<StartedClient | undefined> {
	return new Promise((resolve) => {
		let settled = false
		let webSocket: WebSocket

		const finish = (v: StartedClient | undefined) => {
			if (settled) return
			settled = true
			clearTimeout(timer)
			if (!v) {
				try {
					webSocket?.close()
				} catch {}
			}
			resolve(v)
		}
		// Covers connect AND initialize: both can stall on an unhealthy server.
		const timer = setTimeout(() => {
			console.error(`headlessLsp: ${server.name} did not become ready in ${deadlineMs}ms`)
			finish(undefined)
		}, deadlineMs)

		try {
			webSocket = new WebSocket(server.url)
		} catch (e) {
			console.error(`headlessLsp: could not open a socket to ${server.name}`, e)
			return finish(undefined)
		}

		webSocket.onerror = () => finish(undefined)
		webSocket.onclose = () => finish(undefined)

		webSocket.onopen = async () => {
			const socket = toSocket(webSocket)
			const reader = new WebSocketMessageReader(socket)
			const writer = new WebSocketMessageWriter(socket)

			let resolvePushed: () => void = () => {}
			const pushedDiagnostics = new Promise<void>((r) => (resolvePushed = r))

			const client = new MonacoLanguageClient({
				name: `headless-${server.name}`,
				messageTransports: { reader, writer },
				clientOptions: {
					outputChannel: silentOutputChannel,
					documentSelector: [editorLang],
					errorHandler: {
						error: () => ({ action: ErrorAction.Continue }),
						closed: () => ({ action: CloseAction.DoNotRestart })
					},
					markdown: { isTrusted: true },
					workspaceFolder:
						server.name !== 'deno'
							? { uri: vscode.Uri.parse(uri.toString()), name: 'windmill', index: 0 }
							: undefined,
					initializationOptions: server.initOptions,
					middleware: {
						workspace: {
							configuration: server.middleware ?? (() => [{ enabled: true }])
						},
						// Push servers publish diagnostics even when there are none, which is the only
						// signal that says "this document has been analysed" — markers alone cannot
						// tell not-yet-analysed apart from clean.
						handleDiagnostics: (diagUri, diagnostics, next) => {
							if (diagUri.toString() === uri.toString()) resolvePushed()
							return next(diagUri, diagnostics)
						}
					}
				}
			})

			try {
				await client.start()
			} catch (e) {
				// Language servers ask the client to register global vscode commands. If an
				// editor is already running a client for the same server, those names are taken
				// and this client cannot be used. Reported rather than silently dropped: its
				// diagnostics would be missing from the result.
				console.error(`headlessLsp: ${server.name} client unavailable`, e)
				return finish(undefined)
			}
			finish({ server, client, webSocket, pushedDiagnostics })
		}
	})
}

async function stopClient(started: StartedClient) {
	await withDeadline(
		started.client.stop().catch((e) => console.error('headlessLsp: error stopping client', e)),
		STOP_TIMEOUT_MS,
		undefined
	)
	try {
		started.webSocket.close()
	} catch {}
}

const LSP_TO_MARKER_SEVERITY = {
	1: MarkerSeverity.Error,
	2: MarkerSeverity.Warning,
	3: MarkerSeverity.Info,
	4: MarkerSeverity.Hint
}

function lspDiagnosticToMarker(d: any, uri: Uri): meditor.IMarker {
	return {
		owner: 'headlessLsp',
		resource: uri,
		severity: LSP_TO_MARKER_SEVERITY[d.severity ?? 1] ?? MarkerSeverity.Error,
		message: d.message ?? '',
		source: d.source,
		code: typeof d.code === 'object' ? d.code?.value : d.code,
		// LSP positions are zero-based, Monaco's are one-based.
		startLineNumber: (d.range?.start?.line ?? 0) + 1,
		startColumn: (d.range?.start?.character ?? 0) + 1,
		endLineNumber: (d.range?.end?.line ?? 0) + 1,
		endColumn: (d.range?.end?.character ?? 0) + 1
	} as meditor.IMarker
}

/**
 * Servers implementing LSP 3.17 pull diagnostics (pyright, gopls) publish nothing on
 * their own: the client is expected to ask, and vscode-languageclient only asks for
 * documents visible in an editor. Headless, nothing is visible, so ask directly.
 *
 * Asked of every server rather than only those advertising diagnosticProvider up front,
 * because pyright registers that capability dynamically after initialize. A server that
 * does not support the request simply answers with an error.
 */
async function pullDiagnostics(started: StartedClient, uri: Uri): Promise<meditor.IMarker[]> {
	try {
		const report: any = await started.client.sendRequest('textDocument/diagnostic', {
			textDocument: { uri: uri.toString() }
		})
		const items = report?.items ?? report?.relatedDocuments?.[uri.toString()]?.items ?? []
		return items.map((d: any) => lspDiagnosticToMarker(d, uri))
	} catch (e) {
		console.error(`headlessLsp: ${started.server.name} diagnostic request failed`, e)
		return []
	}
}

// Several servers can report the same problem (and an editor open on the same language
// runs its own clients over the same temporary document), so collapse identical entries.
function dedupe(markers: meditor.IMarker[]): ScriptLintResult {
	const seen = new Set<string>()
	const unique = markers.filter((m) => {
		const key = `${m.severity}:${m.startLineNumber}:${m.startColumn}:${m.message}`
		if (seen.has(key)) return false
		seen.add(key)
		return true
	})
	const errors = unique.filter((m) => m.severity === MarkerSeverity.Error)
	const warnings = unique.filter((m) => m.severity === MarkerSeverity.Warning)
	return { errorCount: errors.length, warningCount: warnings.length, errors, warnings }
}

export interface LspLintResult extends ScriptLintResult {
	/** Configured servers that could not be used, so their diagnostics are missing. */
	unavailableServers: string[]
}

export async function lintWithLsp(req: {
	content: string
	scriptLang: string
	editorLang: string
	workspace: string
	/** Only used to build deno's import map, which resolves relative imports. */
	path?: string
	timeoutMs?: number
}): Promise<LspLintResult> {
	await initializeVscode('headlessLsp')

	const uriString = computeModelUri(
		computeModelPath(undefined, req.scriptLang),
		req.scriptLang,
		req.editorLang
	)
	const uri = Uri.parse(uriString)
	const model = meditor.createModel(req.content, req.editorLang, uri)

	const denoImportMap =
		req.scriptLang === 'deno'
			? buildDenoImportMap(await genAtaRoot(req.workspace), req.path)
			: undefined
	const servers = lspServersFor({
		editorLang: req.editorLang,
		scriptLang: req.scriptLang,
		denoImportMap
	})

	const timeoutMs = req.timeoutMs ?? DEFAULT_TIMEOUT_MS
	const started: StartedClient[] = []
	try {
		const results = await Promise.all(
			servers.map((s) => startClient(s, req.editorLang, uri, timeoutMs))
		)
		for (const r of results) if (r) started.push(r)
		if (started.length === 0) {
			throw new Error(
				`No language server for ${req.scriptLang} responded, so this code could not be linted. The language servers may be unavailable on this instance.`
			)
		}

		// A server is done when it has answered the diagnostic request. If it answers with
		// nothing it may be a publisher instead, so give its push a bounded chance rather
		// than concluding the document is clean.
		const pulled = await withDeadline(
			Promise.all(
				started.map(async (s) => {
					const items = await pullDiagnostics(s, uri)
					if (items.length === 0) {
						await withDeadline(s.pushedDiagnostics, PUSH_FALLBACK_MS, undefined)
					}
					return items
				})
			).then((r) => r.flat()),
			timeoutMs,
			[] as meditor.IMarker[]
		)

		// Pushed diagnostics have already been turned into markers on the model.
		const pushedMarkers = meditor.getModelMarkers({ resource: uri })
		const unavailableServers = servers
			.filter((s) => !started.some((st) => st.server.name === s.name))
			.map((s) => s.name)
		return { ...dedupe([...pushedMarkers, ...pulled]), unavailableServers }
	} finally {
		await Promise.all(started.map(stopClient))
		model.dispose()
	}
}
