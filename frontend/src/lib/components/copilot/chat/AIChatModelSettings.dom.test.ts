import { afterEach, expect, it, vi } from 'vitest'
import { flushSync, mount, unmount } from 'svelte'
import type { AIConfig } from '$lib/gen'
import { loadWorkspaceCopilotInfo, setCopilotInfo } from '$lib/aiStore'
import { AIChatManager } from './AIChatManager.svelte'
import AIChatModelSettings from './AIChatModelSettings.svelte'

vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}))
vi.mock('monaco-editor', () => ({ Selection: class Selection {} }))
vi.mock('$lib/utils/featureUsage', () => ({ logFeatureUsage: vi.fn() }))

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

it("a session's model picker shows its own workspace's model, not the tab's", async () => {
	// The tab shows staging; the session works in its fork.
	setCopilotInfo(CONFIGS.staging)
	const manager = new AIChatManager()
	manager.isSessionChat = true
	manager.workspaceResolver = () => 'fork'
	await loadWorkspaceCopilotInfo('fork')

	component = mount(AIChatModelSettings, {
		target: document.body,
		context: new Map([['aiChatManager', manager]])
	})
	flushSync()

	expect(document.body.textContent).toContain('fork-z')
	expect(document.body.textContent).not.toContain('claude-x')
})
