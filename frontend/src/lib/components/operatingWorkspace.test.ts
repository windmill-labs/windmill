import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const components = dirname(fileURLToPath(import.meta.url))
const lib = dirname(components)

// The editors an AI session mounts for its own, possibly forked, workspace. Everything they
// render acts on that workspace, so a component reading the navigation store directly writes
// to the parent from inside a fork's editor — and nothing at runtime would say so.
const SESSION_EDITORS = [
	'sessions/ScriptEditorView.svelte',
	'sessions/FlowEditorView.svelte',
	'sessions/RawAppEditorView.svelte',
	'sessions/PipelineEditorView.svelte',
	'sessions/PageItemEditorView.svelte'
]

// Components that read the navigation store on purpose, with why. Keep each reason true.
const NAVIGATION_READERS: Record<string, string> = {
	'ScriptBuilder.svelte': 'keeps its own tag list when it acts off the navigation workspace',
	'WorkerTagSelect.svelte': 'the shared worker tag cache is navigation-scoped',
	'WorkerTagPicker.svelte': 'the shared worker tag cache is navigation-scoped',
	'flows/content/FlowModuleWorkerTagSelect.svelte':
		'the shared worker tag cache is navigation-scoped',
	'FolderPicker.svelte': '`$userStore` describes the navigation workspace',
	'FolderEditor.svelte': '`$userStore` describes the navigation workspace'
}

// Followed through plain modules too: a barrel like `$lib/components/common` is how most
// components are reached.
function resolveImport(from: string, spec: string): string | undefined {
	const target = spec.startsWith('$lib/')
		? join(lib, spec.slice('$lib/'.length))
		: spec.startsWith('.')
			? resolve(dirname(from), spec)
			: undefined
	if (!target || target.includes('/gen/')) return undefined
	const candidates = [target, `${target}.ts`, `${target}.svelte.ts`, join(target, 'index.ts')]
	return candidates.find((c) => /\.(svelte|ts|js)$/.test(c) && existsSync(c))
}

function reachableComponents(): string[] {
	const seen = new Set<string>()
	const queue = SESSION_EDITORS.map((f) => join(components, f))
	while (queue.length) {
		const file = queue.pop()!
		if (seen.has(file)) continue
		seen.add(file)
		const source = readFileSync(file, 'utf-8')
		for (const m of source.matchAll(/(?:from\s+|import\(\s*)['"]([^'"]+)['"]/g)) {
			const next = resolveImport(file, m[1])
			if (next) queue.push(next)
		}
	}
	return [...seen].filter((f) => f.endsWith('.svelte')).map((f) => relative(components, f))
}

// Module scripts have no component context to read the operating workspace from, so what
// they fetch takes its workspace explicitly; only instance code and markup are checked.
function readsNavigationStore(file: string): boolean {
	const source = readFileSync(join(components, file), 'utf-8')
		.replace(/<script\b[^>]*\bmodule\b[^>]*>[\s\S]*?<\/script>/g, '')
		.replace(/<!--[\s\S]*?-->|\/\*[\s\S]*?\*\/|(^|[^:])\/\/.*$/gm, '$1')
	return /\$workspaceStore\b|\bget\(workspaceStore\)/.test(source)
}

describe('components under a session editor', () => {
	it('read the operating workspace, not the navigation store', () => {
		const reachable = reachableComponents()
		// A walk that finds nothing would pass vacuously.
		expect(reachable).toContain('triggers/http/RouteEditorInner.svelte')
		const offenders = reachable.filter((f) => !(f in NAVIGATION_READERS) && readsNavigationStore(f))
		expect(offenders).toEqual([])
	})
})
