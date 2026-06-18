import { describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

// aiStore pulls in the (heavy) copilot lib for workspaceAIClients and the gen
// client for WorkspaceService; neither is exercised by setCopilotInfo.
vi.mock('./components/copilot/lib', () => ({
	workspaceAIClients: { init: () => {} }
}))
vi.mock('./gen', () => ({
	WorkspaceService: { getCopilotInfo: async () => ({}) }
}))

import { isWebSearchEnabledForProvider, setCopilotInfo, copilotInfo } from './aiStore'

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
