import { afterEach, expect, it, vi } from 'vitest'
import { flushSync, mount, tick, unmount } from 'svelte'
import type { AIConfig } from '$lib/gen'
import { loadWorkspaceCopilotInfo, setCopilotInfo } from '$lib/aiStore'
import { AIChatManager } from './chat/AIChatManager.svelte'
import CronGen from './CronGen.svelte'
import { getNonStreamingMetadataCompletion } from './lib'

vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}))
vi.mock('monaco-editor', () => ({ Selection: class Selection {} }))
vi.mock('$lib/utils/featureUsage', () => ({ logFeatureUsage: vi.fn() }))
vi.mock('./lib', async (importOriginal) => ({
	...(await importOriginal<typeof import('./lib')>()),
	getNonStreamingMetadataCompletion: vi.fn(async () => '0 0 * * 1')
}))

const CONFIGS: Record<string, AIConfig> = {
	staging: {
		providers: { anthropic: { resource_path: 'u/admin/a', models: ['claude-x'] } },
		default_model: { provider: 'anthropic', model: 'claude-x' }
	},
	fork: {
		providers: { mistral: { resource_path: 'u/admin/m', models: ['fork-z'] } },
		default_model: { provider: 'mistral', model: 'fork-z' }
	}
}

vi.mock('$lib/gen', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/gen')>()),
	WorkspaceService: {
		getCopilotInfo: async ({ workspace }: { workspace: string }) => CONFIGS[workspace]
	}
}))

let component: ReturnType<typeof mount> | undefined
afterEach(() => component && unmount(component))

it("a generator in a session's editor sends with the session's workspace, not the tab's", async () => {
	// The tab's copy holds staging; the session works in its fork.
	setCopilotInfo(CONFIGS.staging)
	const manager = new AIChatManager()
	manager.isSessionChat = true
	manager.workspaceResolver = () => 'fork'
	await loadWorkspaceCopilotInfo('fork')

	component = mount(CronGen, {
		target: document.body,
		props: { schedule: '', cronVersion: 'v2' },
		context: new Map([['aiChatManager', manager]])
	})
	flushSync()
	document.querySelector<HTMLButtonElement>('button[title="AI Assistant"]')!.click()
	await tick()
	const input = document.querySelector<HTMLInputElement>(
		'input[placeholder="CRON schedule description"]'
	)!
	input.value = 'every monday'
	input.dispatchEvent(new Event('input', { bubbles: true }))
	input.dispatchEvent(new KeyboardEvent('keypress', { key: 'Enter', bubbles: true }))
	await tick()

	const target = vi.mocked(getNonStreamingMetadataCompletion).mock.calls[0]?.[2]
	expect(target?.workspace).toBe('fork')
	expect(target?.info.defaultModel?.model).toBe('fork-z')
})
