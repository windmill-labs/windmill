import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

function read(relPath: string): string {
	return readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), relPath), 'utf-8')
}

function navStoreReads(source: string): string[] {
	return source
		.split('\n')
		.map((text, i) => `${i + 1}:${text.trim()}`)
		.filter((l) => l.includes('$workspaceStore'))
}

// The one line allowed to read the nav store: the resolver's fallback, which is what every
// call site outside a detail page still takes.
const FALLBACK =
	/^\d+:let ws = \$derived\((?:workspace \?\? )?detailWs\?\.\(\) \?\? \$workspaceStore\)$/

// A detail component operates on its `workspace` prop, which in an AI session is the session's
// workspace and not the one the top nav points at. Any `$workspaceStore` reintroduced here aims
// that call at the nav workspace, which still looks right on the workspace pages — the only
// place the two coincide — and silently reads the wrong workspace everywhere else.
describe.each(['./ScriptDetail.svelte', './FlowDetail.svelte'])('%s', (relPath) => {
	const source = read(relPath)

	it('never reads the nav workspace store', () => {
		expect(navStoreReads(source)).toEqual([])
	})

	it('registers the resolver for the drawers it opens', () => {
		expect(source).toContain('setDetailWorkspace(() => workspace)')
		expect(source).toContain('setTriggerWorkspace(() => workspace)')
	})
})

// The shared components a detail page opens. Each is mounted from many other places too, so it
// resolves its workspace rather than taking it as a prop: the resolver when a detail page set
// one, the nav store otherwise.
describe.each([
	'../ShareModal.svelte',
	'../MoveDrawer.svelte',
	'../PersistentScriptDrawer.svelte',
	'../ScriptVersionHistory.svelte',
	'../DeployWorkspace.svelte',
	'../SavedInputsPicker.svelte',
	'../HistoricInputs.svelte',
	'../SaveInputsButton.svelte'
])('%s', (relPath) => {
	const source = read(relPath)

	it('reads the nav workspace only as the resolver fallback', () => {
		expect(navStoreReads(source).filter((l) => !FALLBACK.test(l))).toEqual([])
	})

	it('consults the resolver', () => {
		expect(source).toContain('const detailWs = getDetailWorkspace()')
	})
})
