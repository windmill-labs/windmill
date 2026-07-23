import { editor as meditor, Uri } from 'monaco-editor'
import {
	getJavaScriptWorker,
	getTypeScriptWorker
} from '@codingame/monaco-vscode-standalone-typescript-language-features'
import { scriptLangToEditorLang } from '$lib/scripts'
import type { LintOutcome, ScriptLintResult } from '../copilot/chat/shared'
import { initializeVscode, keepModelAroundToAvoidDisposalOfWorkers } from '../vscode'
import {
	setMonacoJavascriptOptions,
	setMonacoTypescriptOptions,
	TS_DIAGNOSTIC_CODES_TO_IGNORE
} from '../monacoLanguagesOptions'
import { computeModelUri, computeOwnedUri } from './monacoUri'
import { readModelMarkers } from './markers'
import {
	acquireOwnedModel,
	disposeOwnedModel,
	readThrough,
	releaseOwnedModel
} from './headlessModelHost'
import {
	ensureCustomWmillTypes,
	ensureResourceTypeNamespace,
	snapshotWorkspaceExtraLibs
} from './typescriptExtraLibs'
import { ataSeedImport, createWindmillAta, genAtaRoot } from './typescriptAta'
import { lintWithLsp } from './headlessLsp'

// Lints code without a mounted editor, by driving the same Monaco model + global
// typescriptDefaults an editor would. Diagnostics belong to the model, not the editor,
// so results match what opening the editor on the same item would show — provided the
// URI and the extra libs are derived through the shared helpers in this directory.

// Languages Monaco's bundled TypeScript worker analyses in-browser.
const TS_WORKER_LANGS = ['bun', 'bunnative', 'nativets', 'tsx', 'javascript', 'jsx']
// Languages analysed by a language server over a websocket.
const LSP_LANGS = ['python3', 'go', 'deno', 'bash']

export function canLintHeadless(scriptLang: string | undefined): boolean {
	const lang = scriptLang ?? ''
	return TS_WORKER_LANGS.includes(lang) || LSP_LANGS.includes(lang)
}

export interface HeadlessLintRequest {
	content: string
	scriptLang: string
	workspace: string
	/** The draft cell this code lives in — its identity, keyed like UserDraft. */
	itemKind: string
	storagePath: string
	/** A unit within the cell: a flow module id or an app runnable key; absent for a whole script. */
	subPath?: string
}

/** The editor path this request maps to — where a mounted editor would put its model. */
function editorPathOf(req: HeadlessLintRequest): string {
	return req.subPath ? `${req.storagePath}/${req.subPath}` : req.storagePath
}

const MAX_ATA_INSTANCES = 4
const absolutePathExtraLibs = new Map<string, { dispose: () => void }>()
const ataByKey = new Map<string, (source: string) => Promise<void>>()
const pendingByUri = new Map<string, Promise<LintOutcome>>()

function withDeadline(promise: Promise<unknown>, ms: number): Promise<unknown> {
	return Promise.race([promise, new Promise((resolve) => setTimeout(resolve, ms))])
}

// Like withDeadline, but reports which happened — so a caller can tell a completed load from a
// timed-out or failed one and treat the latter as inconclusive rather than clean.
function raceLoad(promise: Promise<unknown>, ms: number): Promise<'loaded' | 'failed' | 'timeout'> {
	return Promise.race([
		promise.then(
			() => 'loaded' as const,
			() => 'failed' as const
		),
		new Promise<'timeout'>((resolve) => setTimeout(() => resolve('timeout'), ms))
	])
}

export async function lintCode(
	req: HeadlessLintRequest,
	opts?: { timeoutMs?: number }
): Promise<LintOutcome> {
	if (!canLintHeadless(req.scriptLang)) {
		throw new Error(`Headless linting is not supported for language ${req.scriptLang}`)
	}
	const editorLang = scriptLangToEditorLang(req.scriptLang as any)

	if (LSP_LANGS.includes(req.scriptLang)) {
		// Language servers analyse one document at a time and the editor gives these
		// languages randomized paths, so there is no shared model to serialize on.
		return lintWithLsp({
			content: req.content,
			scriptLang: req.scriptLang,
			editorLang,
			workspace: req.workspace,
			path: editorPathOf(req),
			timeoutMs: opts?.timeoutMs
		})
	}

	const ownedUri = computeOwnedUri(
		{ workspace: req.workspace, itemKind: req.itemKind, storagePath: req.storagePath },
		req.subPath,
		req.scriptLang,
		editorLang
	)

	// Two lints of the same owned model would race on its content; run them in order.
	const previous = pendingByUri.get(ownedUri) ?? Promise.resolve(undefined)
	const run = previous
		.catch(() => undefined)
		.then(() => lintOne(req, ownedUri, editorLang, opts?.timeoutMs ?? 3000))
	pendingByUri.set(
		ownedUri,
		run.finally(() => {
			if (pendingByUri.get(ownedUri) === run) pendingByUri.delete(ownedUri)
		}) as Promise<LintOutcome>
	)
	return run
}

async function lintOne(
	req: HeadlessLintRequest,
	ownedUri: string,
	editorLang: string,
	timeoutMs: number
): Promise<LintOutcome> {
	setMonacoTypescriptOptions()
	if (editorLang === 'javascript') {
		setMonacoJavascriptOptions()
	}
	await initializeVscode('headlessLint')
	keepModelAroundToAvoidDisposalOfWorkers()

	// If an editor in this workspace is showing the item, read its markers — never touch a
	// model it owns. Its markers still have to settle: right after an edit the buffer holds
	// the new content but the worker may not have revalidated yet.
	const editorUri = computeModelUri(editorPathOf(req), req.scriptLang, editorLang)
	const rt = readThrough(editorUri, req.workspace, req.content)
	if (rt) {
		const settled = await waitForMarkersToSettle(Uri.parse(editorUri), editorLang, timeoutMs)
		const result = rt.reread()
		return settled
			? { status: 'complete', result, contentMismatch: rt.contentMismatch }
			: {
					status: 'incomplete',
					result,
					missing: ['the TypeScript checker'],
					contentMismatch: rt.contentMismatch
				}
	}

	// Otherwise lint an owned model.
	const uri = acquireOwnedModel(ownedUri, req.content, editorLang)
	try {
		// Type acquisition writes workspace-agnostic global libs (npm types) and owned-namespace
		// import models, so it stays outside the serialized section below. Bounded: it fetches
		// over the network, and lints of one URI are serialized, so a hung request would wedge
		// the tool for that item.
		if (req.scriptLang === 'bun' || req.scriptLang === 'bunnative' || req.scriptLang === 'tsx') {
			await withDeadline(
				acquireTypes(req, ownedUri).catch((e) =>
					console.error('headlessLint: type acquisition failed', e)
				),
				timeoutMs
			)
		}

		// JavaScript has no workspace-specific declarations; nothing to install or isolate.
		if (editorLang !== 'typescript') {
			return settleAndRead(uri, editorLang, timeoutMs)
		}

		// The resource-type and windmill-client declarations live at global URIs shared with the
		// editor. Install this workspace's, settle+read, then restore — all under one lock so a
		// concurrent lint of another workspace can neither observe nor clobber them.
		return withWorkspaceTypes(async () => {
			const restore = snapshotWorkspaceExtraLibs()
			// Flipped once we stop waiting for the declarations (timeout, failure, or done). A
			// slow request that resolves afterwards checks this before its addExtraLib, so it can
			// never reinstall types over the restored snapshot.
			let cancelled = false
			try {
				const loaded = await raceLoad(
					Promise.all([
						ensureResourceTypeNamespace(req.workspace, req.scriptLang, () => cancelled),
						ensureCustomWmillTypes(req.workspace, () => cancelled)
					]),
					timeoutMs
				)
				// Types that failed or timed out mean the diagnostics were computed without this
				// workspace's declarations — never report that as a clean, complete result.
				if (loaded !== 'loaded') cancelled = true
				const outcome = await settleAndRead(uri, editorLang, timeoutMs)
				if (loaded === 'loaded') return outcome
				return {
					status: 'incomplete',
					result: outcome.result,
					missing: [
						...(outcome.status === 'incomplete' ? outcome.missing : []),
						'the workspace type declarations'
					]
				}
			} finally {
				cancelled = true
				restore()
			}
		})
	} finally {
		releaseOwnedModel(ownedUri)
	}
}

async function settleAndRead(
	uri: Uri,
	editorLang: string,
	timeoutMs: number
): Promise<LintOutcome> {
	const settled = await waitForMarkersToSettle(uri, editorLang, timeoutMs)
	const result = readModelMarkers(uri)
	// A worker that never came up leaves the markers empty for lack of analysis; report it as
	// incomplete rather than let an empty set read as clean.
	return settled
		? { status: 'complete', result }
		: { status: 'incomplete', result, missing: ['the TypeScript checker'] }
}

// Serializes the install → settle → restore of the global workspace declaration libs, so two
// lints of different workspaces can't interleave and leave the wrong declarations resident.
let workspaceTypesChain: Promise<unknown> = Promise.resolve()
function withWorkspaceTypes<T>(run: () => Promise<T>): Promise<T> {
	const task = workspaceTypesChain.catch(() => {}).then(run)
	workspaceTypesChain = task.catch(() => {})
	return task
}

// Known limitation (deferred to the Tier-2 isolated engine): an absolute import like "/f/shared"
// must resolve at the global URI file:///f/shared.ts — TypeScript has nowhere else to look — so
// its declaration is registered in the shared typescriptDefaults and is NOT workspace-isolated.
// A fork linting "/f/shared" whose content differs from the parent can leave a parent-workspace
// editor validating against the fork's version. Narrow, and only a per-lint isolated file system
// fixes it cleanly; see the Tier-2 handoff.
async function acquireTypes(req: HeadlessLintRequest, uriString: string): Promise<void> {
	const key = `${req.workspace}:${uriString}`
	let ata = ataByKey.get(key)
	if (!ata) {
		while (ataByKey.size >= MAX_ATA_INSTANCES) {
			const oldest = ataByKey.keys().next().value
			if (oldest === undefined) break
			ataByKey.delete(oldest)
		}
		ata = await createWindmillAta({
			root: await genAtaRoot(req.workspace),
			scriptPath: editorPathOf(req),
			modelUri: uriString,
			absolutePathExtraLibs,
			// An absolute import like "/f/shared" resolves via new URL() to the canonical editor
			// URI file:///f/shared.ts — outside the __wmlint__ sandbox. If an editor owns that
			// model, ATA must never write the fetched dependency over its (possibly unsaved,
			// possibly other-workspace) buffer. Only create genuinely-missing import models.
			overwriteLocalModels: () => false
		})
		ataByKey.set(key, ata)
		const seed = ataSeedImport(req.scriptLang)
		if (seed) await ata(seed)
	}
	await ata(req.content)
}

async function getWorkerFor(uri: Uri, editorLang: string) {
	const getWorker = editorLang === 'javascript' ? getJavaScriptWorker : getTypeScriptWorker
	// The language mode activates asynchronously once a model of that language exists, so
	// the worker accessor rejects for a short while after the very first model is created.
	const deadline = Date.now() + 5000
	let lastError: unknown
	while (Date.now() < deadline) {
		try {
			const client = await getWorker()
			return await client(uri)
		} catch (e) {
			lastError = e
			await new Promise((r) => setTimeout(r, 200))
		}
	}
	throw lastError ?? new Error('typescript worker unavailable')
}

function normalizeCode(code: unknown): string {
	if (code == null) return ''
	if (typeof code === 'object') return String((code as { value?: unknown }).value ?? '')
	return String(code)
}

interface DiagnosticMessageChain {
	messageText: string
	next?: DiagnosticMessageChain[]
}

// Mirrors TypeScript's flattenDiagnosticMessageText (the exact transform Monaco applies before
// it stores a marker's message), so a worker diagnostic's text can be compared to a published
// marker's message character-for-character.
function flattenTsMessage(
	message: string | DiagnosticMessageChain | undefined,
	indent = 0
): string {
	if (typeof message === 'string') return message
	if (message === undefined) return ''
	let result = ''
	if (indent) {
		result += '\n'
		for (let i = 0; i < indent; i++) result += '  '
	}
	result += message.messageText
	if (message.next) {
		for (const next of message.next) result += flattenTsMessage(next, indent + 1)
	}
	return result
}

// A diagnostic's identity: where it starts, its code, and its message. Position and code alone
// are not enough — an edit that swaps one error for another at the same spot with the same code
// (e.g. two different TS2322 assignability messages) would otherwise look settled while the old
// marker is still showing.
async function expectedMarkerKeys(uri: Uri, editorLang: string): Promise<string[] | undefined> {
	try {
		const worker = await getWorkerFor(uri, editorLang)
		const model = meditor.getModel(uri)
		if (!model) return undefined
		const fileName = uri.toString()
		const [syntactic, semantic, suggestions] = await Promise.all([
			worker.getSyntacticDiagnostics(fileName),
			worker.getSemanticDiagnostics(fileName),
			worker.getSuggestionDiagnostics(fileName)
		])
		return [...syntactic, ...semantic, ...suggestions]
			.filter((d) => !TS_DIAGNOSTIC_CODES_TO_IGNORE.includes(d.code))
			.map((d) => {
				const pos = model.getPositionAt(d.start ?? 0)
				return `${pos.lineNumber}:${pos.column}:${normalizeCode(d.code)}:${flattenTsMessage(d.messageText)}`
			})
			.sort()
	} catch (e) {
		console.error('headlessLint: could not query the typescript worker', e)
		return undefined
	}
}

// Wait until the published markers match the worker's current diagnostics by identity, not
// just by count. Clean code never fires a marker-change event, so there is no event to wait
// on and the worker is the only signal that validation has actually run.
// Returns false when the worker could not be queried at all, or when the markers never
// converged before the deadline: in both cases what is on the model is not a trustworthy
// picture of this content's diagnostics, and the caller must not report it as clean.
async function waitForMarkersToSettle(
	uri: Uri,
	editorLang: string,
	timeoutMs: number
): Promise<boolean> {
	const expected = await expectedMarkerKeys(uri, editorLang)
	if (expected === undefined) return false
	const expectedJoined = expected.join('\n')
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const published = meditor
			.getModelMarkers({ resource: uri, owner: editorLang })
			.map((m) => `${m.startLineNumber}:${m.startColumn}:${normalizeCode(m.code)}:${m.message}`)
			.sort()
			.join('\n')
		if (published === expectedJoined) return true
		await new Promise((r) => setTimeout(r, 50))
	}
	return false
}

const APP_LINTABLE_FILE = /\.(tsx?|jsx?)$/
const APP_DECLARATION_FILE = /\.d\.ts$/

export interface AppFileLintResult {
	filePath: string
	result: ScriptLintResult
}

export interface AppFrontendLintResult {
	files: AppFileLintResult[]
	/** 'incomplete' when a file could not be type-checked; its clean read is not trustworthy. */
	status: 'complete' | 'incomplete'
}

/**
 * Type-checks a raw app's frontend files. Bundling only reports syntax and unresolved
 * imports — esbuild strips types without checking them — so a type error or an undefined
 * variable would otherwise reach the browser unreported.
 *
 * Every file is checked, including ones the entry point never imports: they are still the
 * user's source and show the same problems in the app editor, even though they never ship.
 */
export async function lintAppFrontend(req: {
	appPath: string
	files: Record<string, string>
	workspace: string
	timeoutMs?: number
}): Promise<AppFrontendLintResult> {
	setMonacoTypescriptOptions()
	await initializeVscode('headlessLint')
	keepModelAroundToAvoidDisposalOfWorkers()

	const cell = { workspace: req.workspace, itemKind: 'app', storagePath: req.appPath }
	// One owned model per app file, all in the app's namespace so imports between them
	// resolve the way they do at build time. The in-flight guard keeps them from evicting
	// each other even when the app has more files than the LRU cap.
	const owned: string[] = []
	const reportable: { filePath: string; ownedUri: string }[] = []
	for (const [filePath, content] of Object.entries(req.files)) {
		if (typeof content !== 'string' || !APP_LINTABLE_FILE.test(filePath)) continue
		const ownedUri = computeOwnedUri(cell, filePath.replace(/^\//, ''), undefined, 'typescript')
		acquireOwnedModel(ownedUri, content, 'typescript')
		owned.push(ownedUri)
		// Generated declaration files are not the user's to fix.
		if (!APP_DECLARATION_FILE.test(filePath)) reportable.push({ filePath, ownedUri })
	}

	const timeoutMs = req.timeoutMs ?? 5000
	try {
		const indexUri = computeOwnedUri(cell, 'index.tsx', undefined, 'typescript')
		await withDeadline(
			acquireAppTypes(req.workspace, req.appPath, indexUri, req.files).catch((e) =>
				console.error('headlessLint: app type acquisition failed', e)
			),
			timeoutMs
		)

		const out: AppFileLintResult[] = []
		let incomplete = false
		for (const { filePath, ownedUri } of reportable) {
			const uri = Uri.parse(ownedUri)
			const settled = await waitForMarkersToSettle(uri, 'typescript', timeoutMs)
			if (!settled) incomplete = true
			const result = readModelMarkers(uri)
			if (result.errorCount > 0 || result.warningCount > 0) out.push({ filePath, result })
		}
		return { files: out, status: incomplete ? 'incomplete' : 'complete' }
	} finally {
		for (const ownedUri of owned) disposeOwnedModel(ownedUri)
	}
}

async function acquireAppTypes(
	workspace: string,
	appPath: string,
	indexUri: string,
	files: Record<string, string>
): Promise<void> {
	const key = `app:${workspace}:${appPath}`
	let ata = ataByKey.get(key)
	if (!ata) {
		ata = await createWindmillAta({
			root: await genAtaRoot(workspace),
			scriptPath: appPath,
			modelUri: indexUri,
			absolutePathExtraLibs,
			// The app's own files are already owned models; a relative import like "./foo"
			// resolves to one of them. Never let ATA overwrite an app file with a fetched
			// sibling workspace script of the same name — only create genuinely-missing ones.
			overwriteLocalModels: () => false
		})
		ataByKey.set(key, ata)
	}
	const sources = Object.entries(files)
		.filter(([p, c]) => typeof c === 'string' && APP_LINTABLE_FILE.test(p))
		.map(([, c]) => c)
		.join('\n')
	await ata(sources)
}
