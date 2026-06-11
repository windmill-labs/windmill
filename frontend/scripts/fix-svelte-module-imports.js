// Rewrite extension-less runes-module imports in the svelte-package output.
//
// Source files import runes modules (`foo.svelte.ts`) as `$lib/foo.svelte`,
// which Vite resolves inside this repo by appending `.ts`. svelte-package
// emits those modules as `foo.svelte.js` but keeps the import specifier as
// `../foo.svelte`, which consumers of @windmill-labs/components cannot
// resolve (their bundler looks for a `.svelte` component file that does not
// exist). Append `.js` to any relative `.svelte` specifier whose target only
// exists as `<specifier>.js` in the package output.
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join, resolve } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const packageDir = resolve(__dirname, '..', 'package')

const IMPORT_SPECIFIER_RE = /(\bfrom\s*|\bimport\s*\(?\s*)(['"])(\.\.?\/[^'"]*\.svelte)\2/g

function* walk(dir) {
	for (const entry of readdirSync(dir)) {
		const full = join(dir, entry)
		if (statSync(full).isDirectory()) {
			yield* walk(full)
		} else if (/\.(js|svelte|ts)$/.test(entry)) {
			yield full
		}
	}
}

let rewrittenFiles = 0
let rewrittenImports = 0

for (const file of walk(packageDir)) {
	const content = readFileSync(file, 'utf-8')
	let changed = false
	const updated = content.replace(IMPORT_SPECIFIER_RE, (match, prefix, quote, specifier) => {
		const target = resolve(dirname(file), specifier)
		// A real `.svelte` component file: leave the import alone.
		if (existsSync(target)) {
			return match
		}
		// A runes module emitted as `.svelte.js`: make the extension explicit.
		if (existsSync(target + '.js')) {
			changed = true
			rewrittenImports++
			return `${prefix}${quote}${specifier}.js${quote}`
		}
		return match
	})
	if (changed) {
		writeFileSync(file, updated)
		rewrittenFiles++
	}
}

console.log(
	`fix-svelte-module-imports: rewrote ${rewrittenImports} import(s) in ${rewrittenFiles} file(s)`
)
