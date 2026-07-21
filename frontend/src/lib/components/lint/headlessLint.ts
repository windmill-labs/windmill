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
import { ensureCustomWmillTypes, ensureResourceTypeNamespace } from './typescriptExtraLibs'
import { createWindmillAta, genAtaRoot } from './typescriptAta'
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
		// Bounded: these fetch over the network, and lints of one URI are serialized behind
		// each other, so one hung request would wedge the tool for that item indefinitely.
		if (editorLang === 'typescript') {
			await withDeadline(
				Promise.all([
					ensureResourceTypeNamespace(req.workspace, req.scriptLang),
					ensureCustomWmillTypes(req.workspace)
				]).catch((e) => console.error('headlessLint: extra libs unavailable', e)),
				timeoutMs
			)
		}

		if (req.scriptLang === 'bun' || req.scriptLang === 'bunnative') {
			await withDeadline(
				acquireTypes(req, ownedUri).catch((e) =>
					console.error('headlessLint: type acquisition failed', e)
				),
				timeoutMs
			)
		}

		const settled = await waitForMarkersToSettle(uri, editorLang, timeoutMs)
		const result = readModelMarkers(uri)
		// A worker that never came up leaves the markers empty for lack of analysis; report
		// it as incomplete rather than let an empty set read as clean.
		return settled
			? { status: 'complete', result }
			: { status: 'incomplete', result, missing: ['the TypeScript checker'] }
	} finally {
		releaseOwnedModel(ownedUri)
	}
}

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
			absolutePathExtraLibs
		})
		ataByKey.set(key, ata)
		if (req.scriptLang === 'bun') {
			await ata('import "bun-types"')
		}
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

async function countExpectedMarkers(uri: Uri, editorLang: string): Promise<number | undefined> {
	try {
		const worker = await getWorkerFor(uri, editorLang)
		const fileName = uri.toString()
		const [syntactic, semantic, suggestions] = await Promise.all([
			worker.getSyntacticDiagnostics(fileName),
			worker.getSemanticDiagnostics(fileName),
			worker.getSuggestionDiagnostics(fileName)
		])
		return [...syntactic, ...semantic, ...suggestions].filter(
			(d) => !TS_DIAGNOSTIC_CODES_TO_IGNORE.includes(d.code)
		).length
	} catch (e) {
		console.error('headlessLint: could not query the typescript worker', e)
		return undefined
	}
}

// Ask the worker how many diagnostics it has and wait for that many markers to be
// published. Clean code never fires a marker-change event, so there is no event to wait
// on and a count is the only signal that validation has actually run.
// Returns false when the worker could not be queried at all: the markers then read as
// empty for lack of analysis, not because the code is clean, and the caller must not
// report that as a clean result.
async function waitForMarkersToSettle(
	uri: Uri,
	editorLang: string,
	timeoutMs: number
): Promise<boolean> {
	const expected = await countExpectedMarkers(uri, editorLang)
	if (expected === undefined) return false
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const published = meditor.getModelMarkers({ resource: uri, owner: editorLang }).length
		if (published === expected) return true
		await new Promise((r) => setTimeout(r, 50))
	}
	return true
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
