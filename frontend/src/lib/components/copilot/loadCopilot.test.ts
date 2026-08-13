import { beforeEach, describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

vi.mock('./lib', () => ({
	workspaceAIClients: { init: () => {} }
}))

let requests: { workspace: string; resolve: (config: any) => void }[] = []

vi.mock('$lib/gen', () => ({
	WorkspaceService: {
		getCopilotInfo: ({ workspace }: { workspace: string }) =>
			new Promise((resolve) => requests.push({ workspace, resolve }))
	}
}))

import { loadCopilot } from './loadCopilot'
import { copilotWorkspace } from '$lib/aiStore'

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
		// A per-workspace cache would hand back `a1` here — which has already lost the
		// apply race to `b`, so `a` would silently never be written.
		const a2 = loadCopilot('a')
		expect(requests.map((r) => r.workspace)).toEqual(['a', 'b', 'a'])

		requests[0].resolve(config)
		requests[1].resolve(config)
		requests[2].resolve(config)
		await Promise.all([a1, b, a2])

		expect(get(copilotWorkspace)).toBe('a')
	})
})
