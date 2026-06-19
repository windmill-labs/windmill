import { describe, it, expect, vi } from 'vitest'

// `sessionDraftCodecs` statically imports `initFlowState`, which pulls the flow
// editor's Monaco chain (unloadable under vitest). The script codec never calls
// it; stub the module so the import resolves.
vi.mock('$lib/components/flows/flowState', () => ({ initFlowState: () => Promise.resolve() }))

import { makeScriptCodec } from './sessionDraftCodecs'
import type { SessionRuntime } from './sessionRuntime.svelte'
import type { NewScript } from '$lib/gen'

// Minimal runtime stub: the script codec only touches `runtime.scriptStore.val`.
function runtimeWith(script: Partial<NewScript> & { path: string }): SessionRuntime {
	return { scriptStore: { val: script as NewScript } } as unknown as SessionRuntime
}

const STORAGE = 'u/admin/draft_abc'

describe('makeScriptCodec — path rename', () => {
	it('carries the typed path on the draft so the lists show the friendly name', () => {
		const codec = makeScriptCodec(
			runtimeWith({ path: 'u/admin/friendly', content: 'c', summary: 's' })
		)
		const draft = codec.storeToDraft(undefined)
		expect(draft?.path).toBe('u/admin/friendly')
	})

	it('signature changes on a rename, so the outbound sync persists it', () => {
		const before = makeScriptCodec(runtimeWith({ path: STORAGE, content: 'c' }))
		const after = makeScriptCodec(runtimeWith({ path: 'u/admin/renamed', content: 'c' }))
		expect(before.sig(before.storeToDraft(undefined)!)).not.toBe(
			after.sig(after.storeToDraft(undefined)!)
		)
	})
})
