// Extracts every windmill.dev/docs link referenced in the frontend source and
// verifies none of them 404. Run: `node .github/scripts/check-docs-links.mjs`.
// Used by the check-docs-links GitHub workflow (release / manual trigger only).

import { readdir, readFile } from 'node:fs/promises'
import { join, extname } from 'node:path'

const ROOT = 'frontend/src'
const EXTS = new Set(['.ts', '.js', '.svelte', '.mjs', '.cjs'])
const DOCS_RE = /https?:\/\/(?:www\.)?windmill\.dev\/docs\/[^\s"'`)>\]}]*/g
// `const someBaseUrl = 'https://www.windmill.dev/docs/...'` used later as `${someBaseUrl}/foo`
const BASE_RE = /(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*['"`](https?:\/\/(?:www\.)?windmill\.dev\/docs\/[^'"`]+)['"`]/g

const CONCURRENCY = 24
const TIMEOUT_MS = 20000
const RETRIES = 2

async function walk(dir) {
	const out = []
	for (const entry of await readdir(dir, { withFileTypes: true })) {
		const p = join(dir, entry.name)
		if (entry.isDirectory()) {
			if (entry.name === 'node_modules' || entry.name === '.svelte-kit') continue
			out.push(...(await walk(p)))
		} else if (EXTS.has(extname(entry.name))) {
			out.push(p)
		}
	}
	return out
}

// url (no fragment) -> Set of source files it appears in
const urls = new Map()
const unresolved = []

function record(url, file) {
	const clean = url
		.replace(/\\.*$/, '') // cut at an escape sequence embedded in a string literal (e.g. \n)
		.replace(/#.*$/, '') // drop anchor fragment — irrelevant to a 404 check
		.replace(/[.,;:'")\]]+$/, '')
	if (!clean) return
	// A `{`/`${` means the URL is built from an unresolved template/interpolation var.
	if (clean.includes('{')) {
		unresolved.push(`${clean}  (${file})`)
		return
	}
	if (!urls.has(clean)) urls.set(clean, new Set())
	urls.get(clean).add(file)
}

for (const file of await walk(ROOT)) {
	let content = await readFile(file, 'utf8')
	// Inline file-local base-url constants so `${base}/page` template literals resolve.
	const bases = []
	for (const m of content.matchAll(BASE_RE)) bases.push({ name: m[1], value: m[2], decl: m[0] })
	for (const { name, value } of bases) {
		content = content.replaceAll('${' + name + '}', value)
	}
	// Blank each base declaration so a prefix-only base (no index page of its own,
	// e.g. .../app_configuration_settings) isn't checked as a standalone link.
	// A genuinely bare `${base}` usage was already inlined above, so it's still covered.
	for (const { decl } of bases) content = content.replace(decl, '')
	for (const m of content.matchAll(DOCS_RE)) record(m[0], file)
}

const allUrls = [...urls.keys()].sort()
console.log(`Found ${allUrls.length} distinct docs links across ${ROOT}`)
if (unresolved.length) {
	console.log(`\n⚠️  ${unresolved.length} link(s) built from an unrecognized base URL — skipped (register the base const so they can be checked):`)
	for (const u of [...new Set(unresolved)].sort()) console.log(`   ${u}`)
}

async function check(url) {
	for (let attempt = 0; attempt <= RETRIES; attempt++) {
		const ctrl = new AbortController()
		const timer = setTimeout(() => ctrl.abort(), TIMEOUT_MS)
		try {
			let res = await fetch(url, {
				method: 'HEAD',
				redirect: 'follow',
				signal: ctrl.signal,
				headers: { 'user-agent': 'windmill-docs-link-check' }
			})
			// Some hosts reject HEAD — fall back to GET.
			if (res.status === 405 || res.status === 501) {
				res = await fetch(url, {
					method: 'GET',
					redirect: 'follow',
					signal: ctrl.signal,
					headers: { 'user-agent': 'windmill-docs-link-check' }
				})
			}
			clearTimeout(timer)
			return { url, status: res.status, ok: res.status < 400 }
		} catch (err) {
			clearTimeout(timer)
			if (attempt === RETRIES) return { url, status: 0, ok: false, error: String(err?.message || err) }
			await new Promise((r) => setTimeout(r, 500 * (attempt + 1)))
		}
	}
}

// Simple concurrency pool.
const results = []
let idx = 0
async function worker() {
	while (idx < allUrls.length) {
		const url = allUrls[idx++]
		results.push(await check(url))
	}
}
await Promise.all(Array.from({ length: CONCURRENCY }, worker))

const failures = results.filter((r) => !r.ok)
if (failures.length === 0) {
	console.log(`\n✅ All ${allUrls.length} docs links are reachable.`)
	process.exit(0)
}

console.log(`\n❌ ${failures.length} broken docs link(s):`)
for (const f of failures.sort((a, b) => a.url.localeCompare(b.url))) {
	console.log(`\n  ${f.url}`)
	console.log(`    status: ${f.error ? `error (${f.error})` : f.status}`)
	for (const file of urls.get(f.url)) console.log(`    ↳ ${file}`)
}
process.exit(1)
