import { afterEach, expect, it, vi } from 'vitest'
import { get } from 'svelte/store'
import { flushSync, mount, unmount } from 'svelte'
import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
import { loadCopilot } from '$lib/components/copilot/loadCopilot'
import { workspaceStore } from '$lib/stores'
import { sessionState, type Session } from './sessionState.svelte'
import { openSourceInSession } from './sessionSwitch.svelte'
import { AIChatManager } from '../copilot/chat/AIChatManager.svelte'
import { createCodeSelection } from '../copilot/chat/context'
import SessionWrapper from './SessionWrapper.svelte'

// Measures itself, which jsdom has no layout for. Its content (the chat) is not under test.
vi.mock('svelte-splitpanes', () => ({ Pane: () => {}, Splitpanes: () => {} }))

vi.mock('$lib/navigation', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/navigation')>()),
	goto: vi.fn()
}))

let requests: string[] = []
let version = 1
vi.mock('$lib/gen', async (importOriginal) => {
	const gen = await importOriginal<typeof import('$lib/gen')>()
	return {
		...gen,
		WorkspaceService: Object.assign(Object.create(gen.WorkspaceService), {
			getCopilotInfo: async ({ workspace }: { workspace: string }) => {
				requests.push(workspace)
				return {
					providers: {
						anthropic: { resource_path: 'u/admin/a', models: [`${workspace}-${version}`] }
					}
				}
			}
		})
	}
})

let component: ReturnType<typeof mount> | undefined
afterEach(() => {
	if (component) unmount(component)
	vi.restoreAllMocks()
})

// The page is on `parent`, whose config changes before the session comes on screen.
async function showSession(workspace: string) {
	version = 1
	workspaceStore.set('parent')
	await loadCopilot('parent')
	version = 2
	requests = []
	sessionState.sessions = [
		{ id: 's', summary: 's', workspace_id: workspace, created_at: 0 } as unknown as Session
	]
	component = mount(SessionWrapper, {
		target: document.body,
		props: { sessionId: 's', onScreen: true, previewTabName: () => '' }
	})
	flushSync()
	await vi.waitFor(() => expect(requests).toContain(workspace))
	// Long enough for a refetch that retriggers itself to show.
	await new Promise((r) => setTimeout(r, 50))
	expect(requests).toEqual([workspace])
	expect(get(copilotWorkspace)).toBe('parent')
}

it("a session in another workspace leaves the tab's copy alone when it comes on screen", async () => {
	await showSession('fork')
	expect(get(copilotInfo).aiModels[0]?.model).toBe('parent-1')
})

it("a session in the page's workspace refreshes the tab's copy when it comes on screen", async () => {
	await showSession('parent')
	expect(get(copilotInfo).aiModels[0]?.model).toBe('parent-2')
})

it("sends a new session's first message with the code handed over", async () => {
	const send = vi.spyOn(AIChatManager.prototype, 'sendOrQueue').mockImplementation(() => {})
	const selection = createCodeSelection(
		{ kind: 'script', path: 'f/a/b' },
		{ content: 'return 1', startLine: 2, endLine: 2, lang: 'bun' }
	)
	await openSourceInSession(
		{ target: { kind: 'script', path: 'f/a/b' } },
		{ seedPrompt: 'make it 2', autoSend: true, seedPick: selection }
	)
	const id = sessionState.currentSessionId!
	component = mount(SessionWrapper, {
		target: document.body,
		props: { sessionId: id, onScreen: true, previewTabName: () => '' }
	})
	flushSync()

	await vi.waitFor(() => expect(send).toHaveBeenCalledOnce())
	expect(send).toHaveBeenCalledWith('make it 2', [selection])
	expect(sessionState.sessions.find((s) => s.id === id)?.autoSendPick).toBeUndefined()
})
