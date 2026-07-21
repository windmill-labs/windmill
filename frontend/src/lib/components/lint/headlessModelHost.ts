import { editor as meditor, Uri } from 'monaco-editor'
import type { ScriptLintResult } from '../copilot/chat/shared'

// The single owner of every model the headless linter touches. Two rules:
//   1. When an editor is already showing the item (same URI and workspace), read ITS
//      markers — never create or write.
//   2. Otherwise work in an owned model whose URI is disjoint from every editor's (see
//      OWNED_ROOT in monacoUri.ts), so an owned model can never be one an editor owns.
// Nothing outside this module mutates Monaco's model registry for linting.

interface RegisteredEditor {
	/** Editor's own view of its markers and current buffer, read lazily. */
	read: () => { markers: ScriptLintResult; content: string }
}

// keyed by workspace + editor URI: two editors on the same path in different workspaces
// are distinct registrations, so a fork's editor never shadows the parent's.
const registeredEditors = new Map<string, RegisteredEditor>()

// owned models we created, least-recently-used first; keyed by owned URI string
const ownedModels: string[] = []
const MAX_OWNED_MODELS = 4
// owned models currently mid-lint (awaiting settle); the evictor must not dispose them
const inFlight = new Set<string>()

function editorKey(workspace: string, uri: string): string {
	return `${workspace}\n${uri}`
}

/**
 * An editor announces the URI it owns, its workspace, and how to read its markers/content.
 * Returns a disposer for unmount. This is what makes read-through workspace-correct: a
 * session linting a fork never reads a parent-workspace editor's markers.
 */
export function registerEditor(
	uri: string,
	workspace: string,
	read: () => { markers: ScriptLintResult; content: string }
): () => void {
	const k = editorKey(workspace, uri)
	const entry: RegisteredEditor = { read }
	registeredEditors.set(k, entry)
	return () => {
		if (registeredEditors.get(k) === entry) registeredEditors.delete(k)
	}
}

export interface ReadThrough {
	/** The editor's buffer differs from the content we were asked to lint. */
	contentMismatch: boolean
	/** Re-read the editor's markers (call after waiting for them to settle). */
	reread: () => ScriptLintResult
}

/**
 * If an editor in the same workspace owns `editorUri`, return a handle to its markers (and
 * whether its buffer differs from `requestedContent`). Never creates or writes a model.
 */
export function readThrough(
	editorUri: string,
	workspace: string,
	requestedContent: string
): ReadThrough | undefined {
	const reg = registeredEditors.get(editorKey(workspace, editorUri))
	if (!reg) return undefined
	return {
		contentMismatch: reg.read().content !== requestedContent,
		reread: () => reg.read().markers
	}
}

function touch(uriString: string) {
	const at = ownedModels.indexOf(uriString)
	if (at >= 0) ownedModels.splice(at, 1)
	ownedModels.push(uriString)
}

function evict(keepUri: string) {
	// Bounded by the list length so a set of all-protected models can't spin forever.
	let guard = ownedModels.length
	while (ownedModels.length > MAX_OWNED_MODELS && guard-- > 0) {
		const candidate = ownedModels.shift()
		if (!candidate) continue
		// Protect the model this lint is using and any other lint's in-flight model — a
		// concurrent lint of a different item is still awaiting its markers.
		if (candidate === keepUri || inFlight.has(candidate)) {
			ownedModels.push(candidate)
			continue
		}
		const model = meditor.getModel(Uri.parse(candidate))
		// Owned models are never editor-attached (disjoint namespace); the guard is defence
		// in depth against a future change that violates that.
		if (model && !model.isAttachedToEditor()) model.dispose()
	}
}

/**
 * Ensure an owned model exists at `ownedUri` holding `content`, marking it in-flight so a
 * concurrent lint's eviction cannot dispose it. The caller must call `releaseOwnedModel`
 * when done. Clears stale markers on a content change so a later settle cannot succeed on
 * the previous version's markers.
 */
export function acquireOwnedModel(ownedUri: string, content: string, editorLang: string): Uri {
	inFlight.add(ownedUri)
	const uri = Uri.parse(ownedUri)
	const existing = meditor.getModel(uri)
	if (existing) {
		if (existing.getValue() !== content) {
			existing.setValue(content)
			meditor.setModelMarkers(existing, editorLang, [])
		}
		touch(ownedUri)
	} else {
		meditor.createModel(content, editorLang, uri)
		touch(ownedUri)
	}
	evict(ownedUri)
	return uri
}

export function releaseOwnedModel(ownedUri: string) {
	inFlight.delete(ownedUri)
}
