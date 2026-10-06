// Kept apart from toolCodeDiff.ts, which imports Monaco's diff engine: the chat core runs in
// the app shell and must not statically reach Monaco.
const LANGUAGE_BY_APP_FILE_EXTENSION: Record<string, string> = {
	ts: 'typescript',
	tsx: 'typescript',
	mts: 'typescript',
	js: 'javascript',
	jsx: 'javascript',
	mjs: 'javascript',
	py: 'python',
	css: 'css',
	html: 'xml',
	svelte: 'xml',
	vue: 'xml',
	json: 'json',
	yaml: 'yaml',
	yml: 'yaml'
}

// A raw app's backend runnables are named `backend/<key>/main.ts|py`, so their extension
// picks the language like any frontend file's.
export function appFileEditorLang(filePath: string | undefined): string {
	const extension = filePath?.split('/').pop()?.split('.').slice(1).pop()?.toLowerCase()
	return (extension && LANGUAGE_BY_APP_FILE_EXTENSION[extension]) ?? 'plaintext'
}
