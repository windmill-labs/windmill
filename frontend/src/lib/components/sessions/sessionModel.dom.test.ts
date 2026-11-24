import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import type { AIConfig } from '$lib/gen'
import { copilotSessionModel, invalidateWorkspaceCopilotInfo } from '$lib/aiStore'
import { workspaceStore } from '$lib/stores'
import { createSession, sessionState, type Session } from './sessionState.svelte'
import { getOrCreateRuntime } from './sessionRuntime.svelte'

const mocks = vi.hoisted(() => ({ config: {} as any }))
vi.mock('$lib/utils/featureUsage', () => ({ logFeatureUsage: vi.fn() }))
vi.mock('monaco-editor', () => ({ Selection: class Selection {} }))
vi.mock('$lib/gen', async (importOriginal) => {
	const gen = await importOriginal<typeof import('$lib/gen')>()
	return {
		...gen,
		WorkspaceService: Object.assign(Object.create(gen.WorkspaceService), {
			getCopilotInfo: async () => mocks.config
		})
	}
})

const X = { provider: 'mistral' as const, model: 'model-x' }
const Y = { provider: 'mistral' as const, model: 'model-y' }

function offer(...models: string[]) {
	mocks.config = {
		providers: { mistral: { resource_path: 'u/admin/m', models } },
		default_model: { provider: 'mistral', model: 'model-x' }
	} satisfies AIConfig
	invalidateWorkspaceCopilotInfo()
}

let sent: string[] = []
let nextId = 0
function startedSession() {
	const session = { id: `s${nextId++}`, summary: 's', workspace_id: 'ws', createdAt: 0 }
	sessionState.sessions = [...sessionState.sessions, session as Session]
	const manager = getOrCreateRuntime(session as Session).manager
	const record = () => sessionState.sessions.find((s) => s.id === session.id)
	return { manager, record }
}

beforeEach(() => {
	sent = []
	sessionState.sessions = []
	copilotSessionModel.set(undefined)
	workspaceStore.set('ws')
	offer('model-x', 'model-y')
	vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
		const url = input instanceof Request ? input.url : String(input)
		if (!url.includes('/ai/proxy/')) return Response.json([])
		const body = JSON.parse(String(init?.body ?? '{}'))
		sent.push(body.model)
		const chunk = (delta: object, finish_reason: string | null) =>
			`data: ${JSON.stringify({ id: 'c', object: 'chat.completion.chunk', created: 0, model: body.model, choices: [{ index: 0, delta, finish_reason }] })}\n\n`
		return new Response(
			chunk({ content: '<summary>ok</summary>' }, null) + chunk({}, 'stop') + 'data: [DONE]\n\n',
			{ headers: { 'content-type': 'text/event-stream' } }
		)
	})
})
afterEach(() => vi.unstubAllGlobals())

it('a send keeps the model it was pressed on while the history loads', async () => {
	const a = startedSession()
	const b = startedSession()
	await a.manager.sessionAiTarget()
	let restored = () => {}
	a.manager.setReadyGate(new Promise<void>((r) => (restored = r)))
	const turn = a.manager.sendRequest({ instructions: 'hi' })
	b.manager.pickModel(Y)
	restored()
	await turn
	expect(sent).toEqual(['model-x'])
	expect(a.record()?.model).toMatchObject(X)
})

it('a session sends and compacts with its own pick', async () => {
	const a = startedSession()
	const b = startedSession()
	// Not the workspace default, which a request that loses the pick would fall back to.
	a.manager.pickModel({ ...Y, reasoning: 'high' })
	b.manager.pickModel({ ...X, reasoning: 'low' })
	await a.manager.sendRequest({ instructions: 'hi' })
	await a.manager.sendRequest({ instructions: 'again' })
	await a.manager.compactManually()
	expect(sent).toEqual(['model-y', 'model-y', 'model-y'])
	expect(a.manager.currentModel).toEqual({ ...Y, reasoning: 'high' })
})

it('a pick its workspace no longer offers is replaced for good, and said so', async () => {
	const a = startedSession()
	a.manager.pickModel(Y)
	await a.manager.sessionAiTarget()
	// Only once the send is prepared, as when it commits a workspace that does not offer the pick.
	const beforeSend = a.manager.beforeSend
	a.manager.beforeSend = async () => {
		await beforeSend?.()
		offer('model-x')
	}
	await a.manager.sendRequest({ instructions: 'hi' })
	a.manager.beforeSend = beforeSend

	expect(sent[0]).toBe('model-x')
	offer('model-x', 'model-y')
	await a.manager.sendRequest({ instructions: 'again' })
	expect(sent).toEqual(['model-x', 'model-x'])
	// Later sends keep the notice; a pick ends it.
	expect(a.record()?.replacedModel).toMatchObject(Y)
	a.manager.pickModel(Y)
	expect(a.record()?.replacedModel).toBeUndefined()
})

it('compacting also replaces a pick its workspace no longer offers', async () => {
	const a = startedSession()
	a.manager.pickModel(Y)
	await a.manager.sendRequest({ instructions: 'hi' })
	offer('model-x')
	await a.manager.compactManually()

	expect(sent.at(-1)).toBe('model-x')
	expect(a.record()?.model).toMatchObject(X)
})

it('an untouched draft follows the last pick', async () => {
	const draft = getOrCreateRuntime(createSession()).manager
	const a = startedSession()
	await draft.sessionAiTarget()
	draft.pickModel(X)
	a.manager.pickModel(Y)
	expect(draft.currentModel).toMatchObject(Y)
})
