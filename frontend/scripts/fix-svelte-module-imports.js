// Make the svelte-package output self-contained for consumers.
//
// 1. Runes-module imports: source files import runes modules (`foo.svelte.ts`)
//    as `$lib/foo.svelte`, which Vite resolves inside this repo by appending
//    `.ts`. svelte-package emits those modules as `foo.svelte.js` but keeps
//    the import specifier as `../foo.svelte`, which consumers of
//    @windmill-labs/components cannot resolve (their bundler looks for a
//    `.svelte` component file that does not exist). Append `.js` to any
//    relative `.svelte` specifier whose target only exists as
//    `<specifier>.js` in the package output.
//
// 2. OAuth connect registry: the `$oauth_connect_registry` alias
//    (svelte.config.js) points at `backend/oauth_connect.json`, so the
//    packaged output imports `../../../../backend/oauth_connect.json` — a
//    path that escapes the package root and does not exist in the published
//    tarball. Copy the registry into the package and rewrite those imports
//    to point at it (mirrors how package-system-prompts.js bundles
//    `$system_prompts`).
import { copyFileSync, existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join, relative, resolve } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const packageDir = resolve(__dirname, '..', 'package')
const oauthConnectSrc = resolve(__dirname, '..', '..', 'backend', 'oauth_connect.json')
const oauthConnectDest = join(packageDir, 'oauth_connect.json')

copyFileSync(oauthConnectSrc, oauthConnectDest)

const IMPORT_SPECIFIER_RE = /(\bfrom\s*|\bimport\s*\(?\s*)(['"])(\.\.?\/[^'"]*\.(?:svelte|json))\2/g
const OAUTH_CONNECT_RE = /^(\.\.\/)+backend\/oauth_connect\.json$/

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

function relativeSpecifier(fromFile, toFile) {
	const spec = relative(dirname(fromFile), toFile).replaceAll('\\', '/')
	return spec.startsWith('.') ? spec : `./${spec}`
}

let rewrittenFiles = 0
let rewrittenImports = 0

for (const file of walk(packageDir)) {
	const content = readFileSync(file, 'utf-8')
	let changed = false
	const updated = content.replace(IMPORT_SPECIFIER_RE, (match, prefix, quote, specifier) => {
		if (OAUTH_CONNECT_RE.test(specifier)) {
			changed = true
			rewrittenImports++
			return `${prefix}${quote}${relativeSpecifier(file, oauthConnectDest)}${quote}`
		}
		if (!specifier.endsWith('.svelte')) {
			return match
		}
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
