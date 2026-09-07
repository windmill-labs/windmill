import { describe, expect, it, vi } from 'vitest'

// flowState pulls in the flow-module loaders; only the fetch-generation guard is under test here.
vi.mock('$lib/gen', () => ({ ResourceService: { getResource: vi.fn() } }))
vi.mock('./flowStateUtils.svelte', () => ({ loadFlowModuleState: vi.fn() }))
vi.mock('./utils.svelte', () => ({ emptyFlowModuleState: () => ({}) }))
vi.mock('./agentToolUtils', () => ({
	isFlowModuleTool: () => false,
	agentToolToFlowModule: (t: unknown) => t
}))
vi.mock('$lib/stores', () => ({ workspaceStore: { subscribe: (f: (v: string) => void) => (f('ws'), () => {}) } }))

import {
	claimLinkedToolsFetch,
	invalidateLinkedToolsFetches,
	publishLinkedAgentTools
} from './flowState'
import {
	getLinkedAgentTools,
	linkedToolsScope,
	setLinkedAgentTools
} from './linkedAgentToolsStore.svelte'
import { ResourceService } from '$lib/gen'
import type { AgentTool } from './agentToolUtils'

const tool = (id: string) => ({ id, value: { tool_type: 'flowmodule' } }) as AgentTool

let seq = 0
const scopeFor = (name: string) => linkedToolsScope(`ws${seq++}`, name)

describe('linked tools fetch guard', () => {
	it('lets only the newest fetch for a (scope, module) publish', async () => {
		const scope = scopeFor('flow')
		let release: ((v: unknown) => void) | undefined
		vi.mocked(ResourceService.getResource)
			.mockImplementationOnce(
				() => new Promise((r) => (release = r)) as ReturnType<typeof ResourceService.getResource>
			)
			.mockResolvedValueOnce({ value: { tools: [tool('new')] } } as never)

		const stale = publishLinkedAgentTools('f/a/old', 'ws', scope, 'step')
		await publishLinkedAgentTools('f/a/new', 'ws', scope, 'step')
		release?.({ value: { tools: [tool('old')] } })
		await stale

		expect(getLinkedAgentTools(scope, 'step').map((t) => t.id)).toEqual(['new'])
	})

	// A rename migrates the bucket to a new scope. A fetch still running against the old scope holds
	// a valid generation for that key, so without invalidation it publishes there and the doc-scope
	// sweep carries it forward over the link resolved since.
	it('invalidates every in-flight fetch for a scope', async () => {
		const scope = scopeFor('before-rename')
		let release: ((v: unknown) => void) | undefined
		vi.mocked(ResourceService.getResource).mockImplementationOnce(
			() => new Promise((r) => (release = r)) as ReturnType<typeof ResourceService.getResource>
		)

		const inFlight = publishLinkedAgentTools('f/a/old', 'ws', scope, 'step')
		setLinkedAgentTools(scope, 'step', [tool('kept')], 'u/admin/a')

		invalidateLinkedToolsFetches(scope)
		release?.({ value: { tools: [tool('stale')] } })
		await inFlight

		expect(getLinkedAgentTools(scope, 'step').map((t) => t.id)).toEqual(['kept'])
	})

	it('claiming supersedes an in-flight fetch for that module', async () => {
		const scope = scopeFor('claimed')
		let release: ((v: unknown) => void) | undefined
		vi.mocked(ResourceService.getResource).mockImplementationOnce(
			() => new Promise((r) => (release = r)) as ReturnType<typeof ResourceService.getResource>
		)

		const inFlight = publishLinkedAgentTools('f/a/old', 'ws', scope, 'step')
		claimLinkedToolsFetch(scope, 'step')
		setLinkedAgentTools(scope, 'step', [tool('direct')], 'u/admin/a')
		release?.({ value: { tools: [tool('stale')] } })
		await inFlight

		expect(getLinkedAgentTools(scope, 'step').map((t) => t.id)).toEqual(['direct'])
	})
})
