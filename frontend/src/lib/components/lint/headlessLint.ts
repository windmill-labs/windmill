import { editor as meditor, Uri } from 'monaco-editor'
import {
	getJavaScriptWorker,
	getTypeScriptWorker
} from '@codingame/monaco-vscode-standalone-typescript-language-features'
import { scriptLangToEditorLang } from '$lib/scripts'
import type { ScriptLintResult } from '../copilot/chat/shared'
import { initializeVscode, keepModelAroundToAvoidDisposalOfWorkers } from '../vscode'
import {
	setMonacoJavascriptOptions,
	setMonacoTypescriptOptions,
	TS_DIAGNOSTIC_CODES_TO_IGNORE
} from '../monacoLanguagesOptions'
import { computeModelPath, computeModelUri } from './monacoUri'
import { readModelMarkers } from './markers'
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
	/** Canonical editor path without extension: script path, `<flow>/<moduleId>`, `<app>/<runnableKey>`. */
	path: string
	workspace: string
}

export interface HeadlessLintOutcome extends ScriptLintResult {
	/** An editor already owns this model and its content differs from what was requested. */
	contentMismatch: boolean
	/** Language servers that could not be reached, so their diagnostics are missing. */
	unavailableServers?: string[]
}

// Models we created ourselves, least recently used first.
const ownedModels: string[] = []
const MAX_OWNED_MODELS = 4

const absolutePathExtraLibs = new Map<string, { dispose: () => void }>()
const ataByKey = new Map<string, (source: string) => Promise<void>>()
const pendingByUri = new Map<string, Promise<HeadlessLintOutcome>>()

function withDeadline(promise: Promise<unknown>, ms: number): Promise<unknown> {
	return Promise.race([promise, new Promise((resolve) => setTimeout(resolve, ms))])
}

function touchOwnedModel(uriString: string) {
	const at = ownedModels.indexOf(uriString)
	if (at >= 0) ownedModels.splice(at, 1)
	ownedModels.push(uriString)
}

function evictOwnedModels(keepUri: string) {
	while (ownedModels.length > MAX_OWNED_MODELS) {
		const candidate = ownedModels.shift()
		if (!candidate) continue
		if (candidate === keepUri) {
			// Still in use: keep tracking it, or a later lint would mistake it for a model
			// owned by an editor and refuse to update its content.
			ownedModels.push(candidate)
			continue
		}
		const model = meditor.getModel(Uri.parse(candidate))
		// An editor may have adopted it in the meantime; disposing it would close the
		// user's buffer.
		if (model && !model.isAttachedToEditor()) {
			model.dispose()
		}
	}
}

export async function lintCode(
	req: HeadlessLintRequest,
	opts?: { timeoutMs?: number }
): Promise<HeadlessLintOutcome> {
	if (!canLintHeadless(req.scriptLang)) {
		throw new Error(`Headless linting is not supported for language ${req.scriptLang}`)
	}
	const editorLang = scriptLangToEditorLang(req.scriptLang as any)

	if (LSP_LANGS.includes(req.scriptLang)) {
		// Language servers analyse one document at a time and the editor gives these
		// languages randomized paths, so there is no shared model to serialize on.
		const result = await lintWithLsp({
			content: req.content,
			scriptLang: req.scriptLang,
			editorLang,
			workspace: req.workspace,
			path: req.path,
			timeoutMs: opts?.timeoutMs
		})
		return { ...result, contentMismatch: false }
	}

	const uriString = computeModelUri(
		computeModelPath(req.path, req.scriptLang),
		req.scriptLang,
		editorLang
	)

	// Two lints of the same URI would race on model content; run them in order.
	const previous = pendingByUri.get(uriString) ?? Promise.resolve(undefined)
	const run = previous
		.catch(() => undefined)
		.then(() => lintOne(req, uriString, editorLang, opts?.timeoutMs ?? 3000))
	pendingByUri.set(
		uriString,
		run.finally(() => {
			if (pendingByUri.get(uriString) === run) pendingByUri.delete(uriString)
		}) as Promise<HeadlessLintOutcome>
	)
	return run
}

async function lintOne(
	req: HeadlessLintRequest,
	uriString: string,
	editorLang: string,
	timeoutMs: number
): Promise<HeadlessLintOutcome> {
	setMonacoTypescriptOptions()
	if (editorLang === 'javascript') {
		setMonacoJavascriptOptions()
	}
	await initializeVscode('headlessLint')
	keepModelAroundToAvoidDisposalOfWorkers()

	const uri = Uri.parse(uriString)
	let model = meditor.getModel(uri)
	let contentMismatch = false
	if (model) {
		// Attachment decides, not who created it: an editor can adopt a model we made, and
		// its buffer is what the user sees and would save. Never write to it.
		if (model.isAttachedToEditor() || !ownedModels.includes(uriString)) {
			contentMismatch = model.getValue() !== req.content
		} else if (model.getValue() !== req.content) {
			model.setValue(req.content)
			// Markers from the previous content survive until revalidation finishes, and a
			// matching marker count would let the wait below settle on them.
			meditor.setModelMarkers(model, editorLang, [])
			touchOwnedModel(uriString)
		}
	} else {
		model = meditor.createModel(req.content, editorLang, uri)
		touchOwnedModel(uriString)
		evictOwnedModels(uriString)
	}

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
			acquireTypes(req, uriString).catch((e) =>
				console.error('headlessLint: type acquisition failed', e)
			),
			timeoutMs
		)
	}

	await waitForMarkersToSettle(uri, editorLang, timeoutMs)
	return { ...readModelMarkers(uri), contentMismatch }
}

async function acquireTypes(req: HeadlessLintRequest, uriString: string): Promise<void> {
	const key = `${req.workspace}:${uriString}`
	let ata = ataByKey.get(key)
	if (!ata) {
		while (ataByKey.size >= MAX_OWNED_MODELS) {
			const oldest = ataByKey.keys().next().value
			if (oldest === undefined) break
			ataByKey.delete(oldest)
		}
		ata = await createWindmillAta({
			root: await genAtaRoot(req.workspace),
			scriptPath: req.path,
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
async function waitForMarkersToSettle(uri: Uri, editorLang: string, timeoutMs: number) {
	const expected = await countExpectedMarkers(uri, editorLang)
	if (expected === undefined) return
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const published = meditor.getModelMarkers({ resource: uri, owner: editorLang }).length
		if (published === expected) return
		await new Promise((r) => setTimeout(r, 50))
	}
}
