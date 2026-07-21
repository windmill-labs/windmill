import { createHash as randomHash, langToExt } from '$lib/editorLangUtils'

// Canonical Monaco model path/URI for a piece of Windmill code. Diagnostics are a
// property of the model at a given URI, so any code that lints headlessly must
// derive the URI from here — a URI that differs from the editor's yields a second,
// independently-validated model and diverging markers.

const RANDOMIZED_PATH_LANGS = ['deno', 'go', 'python3']

export function computeModelPath(path: string | undefined, scriptLang: string | undefined): string {
	if (RANDOMIZED_PATH_LANGS.includes(scriptLang ?? '') || path == '' || path == undefined) {
		return randomHash()
	}
	return path
}

/** The file part (path + extension, no leading slash) shared by every URI scheme below. */
function computeFilePart(
	filePath: string,
	scriptLang: string | undefined,
	editorLang: string
): string {
	let file: string
	if (filePath.includes('.')) {
		file = filePath
	} else {
		file = `${filePath}.${scriptLang == 'tsx' ? 'tsx' : langToExt(editorLang)}`
	}
	return file.startsWith('/') ? file.slice(1) : file
}

export function computeModelUri(
	filePath: string,
	scriptLang: string | undefined,
	editorLang: string
): string {
	const file = computeFilePart(filePath, scriptLang, editorLang)
	return !RANDOMIZED_PATH_LANGS.includes(scriptLang ?? '')
		? `file:///${file}`
		: `file:///tmp/monaco/${file}`
}

// A reserved root segment no workspace name or script path can produce, so owned-model
// URIs are disjoint from every editor URI. This is the invariant a headless lint relies on
// to never adopt, overwrite, or dispose a model an editor owns; keep owned URIs under it.
export const OWNED_ROOT = '__wmlint__'

/**
 * URI for a model the headless linter owns, keyed by the draft cell it reflects
 * (workspace + itemKind + storagePath) plus an optional sub-path (a flow module id or an
 * app file). One linted code unit ⇔ one owned model, in a namespace no editor can enter.
 */
export function computeOwnedUri(
	cell: { workspace: string; itemKind: string; storagePath: string },
	subPath: string | undefined,
	scriptLang: string | undefined,
	editorLang: string
): string {
	const inner = subPath ? `${cell.storagePath}/${subPath}` : cell.storagePath
	const file = computeFilePart(inner, scriptLang, editorLang)
	return `file:///${OWNED_ROOT}/${cell.workspace}/${cell.itemKind}/${file}`
}
