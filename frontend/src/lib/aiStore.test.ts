import { describe, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'

let requests: {
	workspace: string
	resolve: (config: any) => void
	reject: (err: unknown) => void
}[] = []

// aiStore pulls in the (heavy) copilot lib for workspaceAIClients; setCopilotInfo does not
// exercise it.
vi.mock('./components/copilot/lib', () => ({
	workspaceAIClients: { init: () => {} }
}))
vi.mock('./gen', () => ({
	WorkspaceService: {
		getCopilotInfo: ({ workspace }: { workspace: string }) =>
			new Promise((resolve, reject) => requests.push({ workspace, resolve, reject }))
	}
}))

import {
	isWebSearchEnabledForProvider,
	setCopilotInfo,
	copilotInfo,
	copilotWorkspace,
	getWorkspaceCopilotInfo,
	invalidateWorkspaceCopilotInfo,
	loadWorkspaceCopilotInfo,
	applySavedWorkspaceCopilotInfo,
	workspaceCopilotInfoFailed
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

// A save hands over the config it wrote: what shows it never blanks, and a load that read the
// config from before the save does not bring it back.
it('a saved config is readable at once and outlives a pre-save load', async () => {
	requests = []
	invalidateWorkspaceCopilotInfo()
	const load = loadWorkspaceCopilotInfo('ws')
	applySavedWorkspaceCopilotInfo('ws', withModel('saved'))
	expect(modelOf(getWorkspaceCopilotInfo('ws'))).toBe('saved')

	requests[0].resolve(withModel('before-save'))
	expect(modelOf(await load)).toBe('saved')
	expect(modelOf(getWorkspaceCopilotInfo('ws'))).toBe('saved')
})

// The tab's copy is the workspace on screen's: a session saving another workspace leaves it alone.
it('a save updates the tab copy only when the tab is on the saved workspace', () => {
	setCopilotInfo(withModel('tab'))
	copilotWorkspace.set('tab-ws')
	applySavedWorkspaceCopilotInfo('fork', withModel('fork'))
	expect(modelOf(get(copilotInfo))).toBe('tab')

	applySavedWorkspaceCopilotInfo('tab-ws', withModel('tab-saved'))
	expect(modelOf(get(copilotInfo))).toBe('tab-saved')
})

// A session waits on its workspace's config to show its transcript; a load that failed must end
// the wait, and the next one that succeeds must clear it.
it('records a failed load until one succeeds', async () => {
	requests = []
	invalidateWorkspaceCopilotInfo()
	const failed = loadWorkspaceCopilotInfo('down')
	requests[0].reject(new Error('500'))
	await expect(failed).rejects.toThrow()
	expect(workspaceCopilotInfoFailed('down')).toBe(true)

	const retried = loadWorkspaceCopilotInfo('down')
	requests[1].resolve(withModel('back'))
	expect(modelOf(await retried)).toBe('back')
	expect(workspaceCopilotInfoFailed('down')).toBe(false)
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
