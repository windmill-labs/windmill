import { describe, expect, it } from 'vitest'
import {
	clearLinkedAgentTools,
	getLinkedAgentTools,
	linkedToolsScope,
	migrateLinkedAgentToolsScope,
	releaseLinkedToolsScope,
	retainLinkedToolsScope,
	setLinkedAgentTools
} from './linkedAgentToolsStore.svelte'
import type { AgentTool } from './agentToolUtils'

// The cap in the store. Filling past it is the only way to observe eviction.
const MAX_SCOPES = 32

function tool(id: string): AgentTool {
	return { id, value: { tool_type: 'flowmodule', type: 'script', path: `u/t/${id}` } } as AgentTool
}

// Unique per test so the module-level store doesn't leak between them.
let seq = 0
function freshScope(name = 'flow') {
	return linkedToolsScope(`ws${seq++}`, name)
}

function fillPastCap(exclude: string) {
	for (let i = 0; i <= MAX_SCOPES; i++) {
		const filler = linkedToolsScope('filler', `${exclude}-${seq}-${i}`)
		setLinkedAgentTools(filler, 'm', [tool(`f${i}`)])
	}
}

describe('linkedAgentToolsStore', () => {
	it('keeps each scope module map separate', () => {
		const a = freshScope()
		const b = freshScope()
		setLinkedAgentTools(a, 'step', [tool('x')])
		setLinkedAgentTools(b, 'step', [tool('y')])
		expect(getLinkedAgentTools(a, 'step').map((t) => t.id)).toEqual(['x'])
		expect(getLinkedAgentTools(b, 'step').map((t) => t.id)).toEqual(['y'])
	})

	it('evicts an unretained scope once past the cap', () => {
		const victim = freshScope()
		setLinkedAgentTools(victim, 'step', [tool('x')])
		fillPastCap(victim)
		expect(getLinkedAgentTools(victim, 'step')).toEqual([])
	})

	// A run viewer holds one scope per mounted nested job; those must not evict what is on screen.
	it('never evicts a retained scope', () => {
		const held = freshScope()
		setLinkedAgentTools(held, 'step', [tool('x')])
		retainLinkedToolsScope(held)
		fillPastCap(held)
		expect(getLinkedAgentTools(held, 'step').map((t) => t.id)).toEqual(['x'])
		releaseLinkedToolsScope(held)
	})

	// Retained scopes can push the store past the cap, since eviction skips them. Releasing one has
	// to prune, or closing views would leave it over the cap for the tab's life.
	it('prunes on release once retained scopes have filled the cap', () => {
		const held: string[] = []
		for (let i = 0; i <= MAX_SCOPES; i++) {
			const scope = linkedToolsScope('retained', `${seq}-${i}`)
			setLinkedAgentTools(scope, 'step', [tool(`t${i}`)])
			retainLinkedToolsScope(scope)
			held.push(scope)
		}
		// All retained, so nothing was evicted and the store sits over the cap.
		expect(getLinkedAgentTools(held[0], 'step').map((t) => t.id)).toEqual(['t0'])

		releaseLinkedToolsScope(held[0])
		expect(getLinkedAgentTools(held[0], 'step')).toEqual([])
		expect(getLinkedAgentTools(held[held.length - 1], 'step')).toHaveLength(1)

		for (const scope of held.slice(1)) {
			releaseLinkedToolsScope(scope)
		}
	})

	// A rename moves readers to a new scope; they only retain the new key afterwards, so the
	// migration itself must not let the fresh bucket be evicted.
	it('carries tools across a rename without evicting the new scope', () => {
		const from = freshScope('old')
		const to = freshScope('new')
		setLinkedAgentTools(from, 'step', [tool('x')])
		retainLinkedToolsScope(from)
		migrateLinkedAgentToolsScope(from, to)
		expect(getLinkedAgentTools(to, 'step').map((t) => t.id)).toEqual(['x'])
		expect(getLinkedAgentTools(from, 'step')).toEqual([])
		releaseLinkedToolsScope(from)
	})

	// Readers release the old scope before retaining the new one. Over the cap, with everything else
	// retained, the migrated bucket would be the only evictable entry in that window.
	it('keeps migrated tools when the old scope is released before the new one is retained', () => {
		const held: string[] = []
		for (let i = 0; i < MAX_SCOPES; i++) {
			const scope = linkedToolsScope('rename-fill', `${seq}-${i}`)
			setLinkedAgentTools(scope, 'step', [tool(`t${i}`)])
			retainLinkedToolsScope(scope)
			held.push(scope)
		}
		const from = freshScope('before')
		const to = freshScope('after')
		setLinkedAgentTools(from, 'step', [tool('x')])
		retainLinkedToolsScope(from)

		migrateLinkedAgentToolsScope(from, to)
		releaseLinkedToolsScope(from)
		retainLinkedToolsScope(to)

		expect(getLinkedAgentTools(to, 'step').map((t) => t.id)).toEqual(['x'])

		releaseLinkedToolsScope(to)
		for (const scope of held) {
			releaseLinkedToolsScope(scope)
		}
	})

	it('clears one module without disturbing its siblings', () => {
		const scope = freshScope()
		setLinkedAgentTools(scope, 'a', [tool('x')])
		setLinkedAgentTools(scope, 'b', [tool('y')])
		clearLinkedAgentTools(scope, 'a')
		expect(getLinkedAgentTools(scope, 'a')).toEqual([])
		expect(getLinkedAgentTools(scope, 'b').map((t) => t.id)).toEqual(['y'])
	})
})
