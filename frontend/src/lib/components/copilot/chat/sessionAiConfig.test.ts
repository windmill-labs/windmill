import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AIConfig } from '$lib/gen'
import { copilotSessionModel, setCopilotInfo } from '$lib/aiStore'
import { workspaceStore } from '$lib/stores'
import { workspaceAIClients } from '../lib'
import { AIChatManager } from './AIChatManager.svelte'
import { generateSessionSummary } from '$lib/components/sessions/sessionTitle'

vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}))
vi.mock('monaco-editor', () => ({ Selection: class Selection {} }))
vi.mock('$lib/utils/featureUsage', () => ({ logFeatureUsage: vi.fn() }))

// The session works in `staging` while the tab shows `prod`: every request the session
// makes must carry staging's proxy, model and limits.
const CONFIGS: Record<string, AIConfig> = {
	staging: {
		providers: { mistral: { resource_path: 'u/admin/mistral', models: ['staging-model'] } },
		default_model: { provider: 'mistral', model: 'staging-model' },
		max_tokens_per_model: { 'mistral:staging-model': 3000 },
		context_window_per_model: { 'mistral:staging-model': 30000 }
	},
	prod: {
		providers: { deepseek: { resource_path: 'u/admin/deepseek', models: ['prod-model'] } },
		default_model: { provider: 'deepseek', model: 'prod-model' }
	}
}

vi.mock('$lib/gen', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/gen')>()),
	WorkspaceService: {
		getCopilotInfo: async ({ workspace }: { workspace: string }) => CONFIGS[workspace]
	}
}))

type SentRequest = { url: string; body: any }
let sent: SentRequest[] = []

function completionResponse(body: any): Response {
	const text = '<summary>Earlier work</summary>'
	if (!body.stream) {
		return Response.json({
			id: 'c',
			object: 'chat.completion',
			created: 0,
			model: body.model,
			choices: [
				{
					index: 0,
					message: { role: 'assistant', content: 'Staging title' },
					finish_reason: 'stop'
				}
			]
		})
	}
	const chunk = (delta: object, finish_reason: string | null) =>
		`data: ${JSON.stringify({
			id: 'c',
			object: 'chat.completion.chunk',
			created: 0,
			model: body.model,
			choices: [{ index: 0, delta, finish_reason }]
		})}\n\n`
	return new Response(chunk({ content: text }, null) + chunk({}, 'stop') + 'data: [DONE]\n\n', {
		headers: { 'content-type': 'text/event-stream' }
	})
}

function sessionManager(): AIChatManager {
	const manager = new AIChatManager()
	manager.isSessionChat = true
	manager.workspaceResolver = () => 'staging'
	manager.messages = [
		{ role: 'user', content: 'q1' },
		{ role: 'assistant', content: 'a1' },
		{ role: 'user', content: 'q2' },
		{ role: 'assistant', content: 'a2' },
		{ role: 'user', content: 'q3' }
	]
	manager.displayMessages = [
		{ role: 'user', content: 'q1', index: 0 },
		{ role: 'assistant', content: 'a1' },
		{ role: 'user', content: 'q2', index: 2 },
		{ role: 'assistant', content: 'a2' },
		{ role: 'user', content: 'q3', index: 4 }
	] as AIChatManager['displayMessages']
	return manager
}

function expectSentForStaging(request: SentRequest | undefined) {
	expect(request?.url).toContain('/w/staging/ai/proxy/')
	expect(request?.body.model).toBe('staging-model')
	expect(request?.body.max_tokens).toBe(3000)
}

beforeEach(() => {
	sent = []
	vi.stubGlobal('location', { origin: 'http://localhost' })
	vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
		const url = input instanceof Request ? input.url : String(input)
		const body = JSON.parse(
			String(init?.body ?? (input instanceof Request ? await input.text() : '{}'))
		)
		sent.push({ url, body })
		return completionResponse(body)
	})
	// The tab is on prod, with prod's model picked.
	workspaceStore.set('prod')
	workspaceAIClients.init('prod')
	setCopilotInfo(CONFIGS.prod)
	copilotSessionModel.set({ provider: 'deepseek', model: 'prod-model' })
})

afterEach(() => {
	vi.unstubAllGlobals()
})

describe('an AI session in another workspace than the tab', () => {
	it('names the session with its own workspace', async () => {
		expect(await generateSessionSummary(sessionManager())).toBe('Staging title')
		expect(sent).toHaveLength(1)
		expectSentForStaging(sent[0])
	})

	it('summarizes for compaction with its own workspace', async () => {
		const manager = sessionManager()
		await manager.compactManually()
		expect(sent).toHaveLength(1)
		expectSentForStaging(sent[0])
	})

	it("compacts at its own workspace's context window", async () => {
		const manager = sessionManager()
		manager.abortController = new AbortController()
		// 80% of staging's 30k window; far below the window prod's model would get.
		const compacted = await (manager as any).compactIfOverTrigger(25000, {
			provider: 'mistral',
			model: 'staging-model'
		})
		expect(compacted).toBe(true)
	})
})
