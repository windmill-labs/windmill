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
//
// 3. Chat SDK: the `windmill-chat` alias (svelte.config.js) points at
//    `chat-sdk/src/index.ts`, which escapes the package root the same way.
//    Compile that entry point into the package and rewrite those imports.
//
// 4. `$lib/utils` from inside `utils/`: svelte-package turns it into `./`,
//    which is the `utils/` directory (no index) rather than `utils.js`.
//    Point a directory specifier with no index at its sibling module.
//
// A relative import that still escapes the package root, or an
// `import.meta.glob` left on an alias, fails the build: either one
// publishes fine and only breaks in the consumer's bundler.
import { execFileSync } from 'child_process'
import { copyFileSync, existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join, relative, resolve, sep } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const packageDir = resolve(__dirname, '..', 'package')
const oauthConnectSrc = resolve(__dirname, '..', '..', 'backend', 'oauth_connect.json')
const oauthConnectDest = join(packageDir, 'oauth_connect.json')

copyFileSync(oauthConnectSrc, oauthConnectDest)

const chatSdkSrc = resolve(__dirname, '..', '..', 'chat-sdk', 'src', 'index.ts')
const chatSdkDest = join(packageDir, 'chat-sdk')

// File arguments make tsc ignore every tsconfig and emit only what the entry
// point reaches, which leaves out the React and AI SDK adapters.
execFileSync(
	'npx',
	[
		'tsc',
		chatSdkSrc,
		'--outDir',
		chatSdkDest,
		'--declaration',
		'--target',
		'ES2021',
		'--module',
		'ESNext',
		'--moduleResolution',
		'bundler',
		'--skipLibCheck'
	],
	{ stdio: 'inherit', cwd: resolve(__dirname, '..') }
)

const IMPORT_SPECIFIER_RE = /(\bfrom\s*|\bimport\s*\(?\s*)(['"])(\.\.?(?:\/[^'"]*)?)\2/g
const OAUTH_CONNECT_RE = /^(\.\.\/)+backend\/oauth_connect\.json$/
const CHAT_SDK_RE = /^(\.\.\/)+chat-sdk\/src\/index\.ts$/
const DIRECTORY_RE = /^\.\.?$|\/$/
const ALIASED_GLOB_RE = /import\.meta\.glob\(\s*['"]\$/

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
const unresolvable = []

for (const file of walk(packageDir)) {
	const content = readFileSync(file, 'utf-8')
	if (ALIASED_GLOB_RE.test(content)) {
		unresolvable.push(`${relative(packageDir, file)}: import.meta.glob on an alias`)
	}
	let changed = false
	const updated = content.replace(IMPORT_SPECIFIER_RE, (match, prefix, quote, specifier) => {
		if (OAUTH_CONNECT_RE.test(specifier)) {
			changed = true
			rewrittenImports++
			return `${prefix}${quote}${relativeSpecifier(file, oauthConnectDest)}${quote}`
		}
		if (CHAT_SDK_RE.test(specifier)) {
			changed = true
			rewrittenImports++
			return `${prefix}${quote}${relativeSpecifier(file, join(chatSdkDest, 'index.js'))}${quote}`
		}
		const target = resolve(dirname(file), specifier)
		if (target !== packageDir && !target.startsWith(packageDir + sep)) {
			unresolvable.push(`${relative(packageDir, file)}: ${specifier}`)
			return match
		}
		if (DIRECTORY_RE.test(specifier)) {
			if (!existsSync(join(target, 'index.js')) && existsSync(target + '.js')) {
				changed = true
				rewrittenImports++
				return `${prefix}${quote}${relativeSpecifier(file, target + '.js')}${quote}`
			}
			return match
		}
		if (!specifier.endsWith('.svelte')) {
			return match
		}
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

if (unresolvable.length > 0) {
	console.error(
		`fix-svelte-module-imports: ${unresolvable.length} import(s) a consumer cannot resolve:\n` +
			unresolvable.join('\n')
	)
	process.exit(1)
}

console.log(
	`fix-svelte-module-imports: rewrote ${rewrittenImports} import(s) in ${rewrittenFiles} file(s)`
)
