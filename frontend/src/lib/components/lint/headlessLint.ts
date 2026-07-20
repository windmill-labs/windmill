import { editor as meditor, MarkerSeverity, Uri } from 'monaco-editor'
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
import { ensureCustomWmillTypes, ensureResourceTypeNamespace } from './typescriptExtraLibs'
import { createWindmillAta, genAtaRoot } from './typescriptAta'

// Lints code without a mounted editor, by driving the same Monaco model + global
// typescriptDefaults an editor would. Diagnostics belong to the model, not the editor,
// so results match what opening the editor on the same item would show — provided the
// URI and the extra libs are derived through the shared helpers in this directory.

const HEADLESS_LINT_LANGS = ['bun', 'bunnative', 'nativets', 'tsx', 'javascript', 'jsx']

export function canLintHeadless(scriptLang: string | undefined): boolean {
	return HEADLESS_LINT_LANGS.includes(scriptLang ?? '')
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
}

export function readModelMarkers(uri: Uri): ScriptLintResult {
	const markers = meditor.getModelMarkers({ resource: uri })
	const errors = markers.filter((m) => m.severity === MarkerSeverity.Error)
	const warnings = markers.filter((m) => m.severity === MarkerSeverity.Warning)
	return { errorCount: errors.length, warningCount: warnings.length, errors, warnings }
}

// Models we created ourselves, oldest first. An editor may later adopt one by URI, so
// eviction must never dispose a model that is attached to an editor.
const ownedModels: string[] = []
const MAX_OWNED_MODELS = 4

const absolutePathExtraLibs = new Map<string, { dispose: () => void }>()
const ataByKey = new Map<string, (source: string) => Promise<void>>()
const pendingByUri = new Map<string, Promise<HeadlessLintOutcome>>()

function evictOwnedModels(keepUri: string) {
	while (ownedModels.length > MAX_OWNED_MODELS) {
		const candidate = ownedModels.shift()
		if (!candidate || candidate === keepUri) continue
		const model = meditor.getModel(Uri.parse(candidate))
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
		const isOurs = ownedModels.includes(uriString)
		if (isOurs) {
			if (model.getValue() !== req.content) model.setValue(req.content)
		} else {
			// An editor owns this model: its content is what the user sees and what they would
			// see the markers for. Lint it as-is rather than overwriting their buffer.
			contentMismatch = model.getValue() !== req.content
		}
	} else {
		model = meditor.createModel(req.content, editorLang, uri)
		ownedModels.push(uriString)
		evictOwnedModels(uriString)
	}

	if (editorLang === 'typescript') {
		await Promise.all([
			ensureResourceTypeNamespace(req.workspace, req.scriptLang).catch((e) =>
				console.error('headlessLint: resource types unavailable', e)
			),
			ensureCustomWmillTypes(req.workspace).catch(() => undefined)
		])
	}

	if (req.scriptLang === 'bun' || req.scriptLang === 'bunnative') {
		await acquireTypes(req, uriString).catch((e) =>
			console.error('headlessLint: type acquisition failed', e)
		)
	}

	await waitForMarkersToSettle(uri, editorLang, timeoutMs)
	return { ...readModelMarkers(uri), contentMismatch }
}

async function acquireTypes(req: HeadlessLintRequest, uriString: string): Promise<void> {
	const key = `${req.workspace}:${uriString}`
	let ata = ataByKey.get(key)
	if (!ata) {
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

// The editor's own lint paths sleep a fixed delay and hope. Instead, ask the worker how
// many diagnostics it has and wait for that many markers to be published — which also
// terminates promptly for clean code, where no marker-change event ever fires.
async function waitForMarkersToSettle(uri: Uri, editorLang: string, timeoutMs: number) {
	const expected = await countExpectedMarkers(uri, editorLang)
	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		const published = meditor.getModelMarkers({ resource: uri, owner: editorLang }).length
		if (expected !== undefined && published === expected) return
		await new Promise((r) => setTimeout(r, 50))
	}
}
