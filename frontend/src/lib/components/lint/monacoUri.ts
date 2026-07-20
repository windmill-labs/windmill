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

export function computeModelUri(
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
	if (file.startsWith('/')) {
		file = file.slice(1)
	}
	return !RANDOMIZED_PATH_LANGS.includes(scriptLang ?? '')
		? `file:///${file}`
		: `file:///tmp/monaco/${file}`
}
