import { describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

let requests: { workspace: string; resolve: (config: any) => void }[] = []

// aiStore pulls in the (heavy) copilot lib for workspaceAIClients; setCopilotInfo does not
// exercise it.
vi.mock('./components/copilot/lib', () => ({
	workspaceAIClients: { init: () => {} }
}))
vi.mock('./gen', () => ({
	WorkspaceService: {
		getCopilotInfo: ({ workspace }: { workspace: string }) =>
			new Promise((resolve) => requests.push({ workspace, resolve }))
	}
}))

import {
	isWebSearchEnabledForProvider,
	setCopilotInfo,
	copilotInfo,
	getWorkspaceCopilotInfo,
	invalidateWorkspaceCopilotInfo,
	loadWorkspaceCopilotInfo
} from './aiStore'

const withModel = (model: string) => ({
	providers: { anthropic: { resource_path: 'u/admin/anthropic', models: [model] } }
})
const modelOf = (info: { aiModels: { model: string }[] } | undefined) => info?.aiModels[0]?.model

// A config saved, or one every workspace inherits, while a session's load is in flight: the load
// read the config from before the save and must not keep it.
describe.each([
	['its workspace', () => invalidateWorkspaceCopilotInfo('ws')],
	['every workspace', () => invalidateWorkspaceCopilotInfo()]
])('loadWorkspaceCopilotInfo invalidated for %s mid-fetch', (_, invalidate) => {
	it('refetches instead of keeping the outdated config', async () => {
		requests = []
		invalidateWorkspaceCopilotInfo()
		const load = loadWorkspaceCopilotInfo('ws')
		invalidate()
		requests[0].resolve(withModel('before-save'))
		await vi.waitFor(() => expect(requests).toHaveLength(2))
		requests[1].resolve(withModel('saved'))

		expect(modelOf(await load)).toBe('saved')
		expect(modelOf(getWorkspaceCopilotInfo('ws'))).toBe('saved')
	})
})

describe('setCopilotInfo legacy /thinking migration', () => {
	it('strips the /thinking suffix from configured model slots', () => {
		setCopilotInfo({
			providers: {
				anthropic: {
					resource_path: 'u/admin/anthropic',
					models: ['claude-sonnet-4-6', 'claude-sonnet-4-6/thinking']
				}
			},
			default_model: { model: 'claude-sonnet-4-6/thinking', provider: 'anthropic' },
			metadata_model: { model: 'claude-sonnet-4-6/thinking', provider: 'anthropic' },
			code_completion_model: { model: 'claude-3-5-haiku-latest', provider: 'anthropic' }
		})

		const info = get(copilotInfo)
		// configured slots no longer carry the deprecated suffix
		expect(info.defaultModel?.model).toBe('claude-sonnet-4-6')
		expect(info.metadataModel?.model).toBe('claude-sonnet-4-6')
		expect(info.codeCompletionModel?.model).toBe('claude-3-5-haiku-latest')
		// the model list is stripped and deduped
		expect(info.aiModels.map((m) => m.model)).toEqual(['claude-sonnet-4-6'])
	})

	it('keeps the models but turns the assistant off when the workspace disabled it', () => {
		setCopilotInfo({
			providers: {
				anthropic: {
					resource_path: 'u/admin/anthropic',
					models: ['claude-sonnet-4-6']
				}
			},
			copilot_disabled: true
		})

		const info = get(copilotInfo)
		expect(info.enabled).toBe(false)
		expect(info.workspaceDisabled).toBe(true)
		// The providers still describe what AI agent steps can run on.
		expect(info.aiModels.map((m) => m.model)).toEqual(['claude-sonnet-4-6'])
	})

	it('defaults provider web search on unless explicitly disabled', () => {
		setCopilotInfo({
			providers: {
				openai: {
					resource_path: 'u/admin/openai',
					models: ['gpt-4.1'],
					web_search_enabled: false
				},
				anthropic: {
					resource_path: 'u/admin/anthropic',
					models: ['claude-sonnet-4-6']
				}
			}
		})

		expect(isWebSearchEnabledForProvider('openai')).toBe(false)
		expect(isWebSearchEnabledForProvider('anthropic')).toBe(true)
	})
})
