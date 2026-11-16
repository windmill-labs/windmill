import { beforeEach, describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

let requests: { workspace: string; resolve: (config: any) => void }[] = []

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		getCopilotInfo: ({ workspace }: { workspace: string }) =>
			new Promise((resolve) => requests.push({ workspace, resolve }))
	}
}))

import { loadCopilot } from './loadCopilot'
import {
	copilotInfo,
	copilotWorkspace,
	getWorkspaceCopilotInfo,
	loadWorkspaceCopilotInfo
} from '$lib/aiStore'

const config = { providers: { anthropic: { resource_path: 'u/admin/anthropic', models: ['m'] } } }

describe('loadCopilot', () => {
	beforeEach(() => {
		requests = []
		copilotWorkspace.set(undefined)
	})

	it('shares one request between callers loading the same workspace', async () => {
		const first = loadCopilot('ws')
		const second = loadCopilot('ws')

		expect(requests.map((r) => r.workspace)).toEqual(['ws'])
		requests[0].resolve(config)
		await Promise.all([first, second])
		expect(get(copilotWorkspace)).toBe('ws')
	})

	it('lets a re-requested workspace win back over the one that superseded it', async () => {
		const a1 = loadCopilot('a')
		const b = loadCopilot('b')
		// `a2` may share `a1`'s download but must apply it on its own token: `a1` already lost
		// the apply race to `b`, so handing back `a1` itself would leave `a` never written.
		const a2 = loadCopilot('a')
		expect(requests.map((r) => r.workspace)).toEqual(['a', 'b'])

		requests[0].resolve(config)
		requests[1].resolve(config)
		await Promise.all([a1, b, a2])

		expect(get(copilotWorkspace)).toBe('a')
	})

	// The config can change elsewhere (another tab, another admin): a refresh of the tab's config
	// must also be what a session in that workspace sends with.
	it('refreshes the workspace config sessions send with, keeping the old one until then', async () => {
		const withModel = (model: string) => ({
			providers: { anthropic: { resource_path: 'u/admin/anthropic', models: [model] } }
		})
		const sessionLoad = loadWorkspaceCopilotInfo('refreshed')
		requests[0].resolve(withModel('old'))
		await sessionLoad
		const refresh = loadCopilot('refreshed')
		await vi.waitFor(() => expect(requests).toHaveLength(2))
		expect(getWorkspaceCopilotInfo('refreshed')?.aiModels[0]?.model).toBe('old')
		requests[1].resolve(withModel('new'))
		await refresh

		expect(get(copilotInfo).aiModels[0]?.model).toBe('new')
		expect(getWorkspaceCopilotInfo('refreshed')?.aiModels[0]?.model).toBe('new')
	})
})
