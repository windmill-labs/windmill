import { parse_ts_imports } from 'windmill-parser-wasm-ts'

export function parseTypescriptDeps(code: string): string[] {
	let r = JSON.parse(parse_ts_imports(code))
	if (r.error) {
		console.error(r.error)
		return []
	} else {
		return r.imports
	}
}

export function isTypescriptRelativePath(d: string) {
	return (
		d.startsWith('./') ||
		d.startsWith('../') ||
		d.startsWith('/') ||
		d.startsWith('.../') ||
		d.startsWith('/')
	)
}

export function approximateFindPythonRelativePath(code: string) {
	// Define the regular expression for finding relative imports
	const regex = /^\s*from\s+(\.+)\w*\s+import\s+.+$/gm

	// Use match to find all matches in the code
	const matches = code.match(regex)
	return [...(matches?.entries() ?? [])]
}
