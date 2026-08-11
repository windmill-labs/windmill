import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import type { Tool } from '../shared'

// The real ../shared pulls the whole component/monaco graph, which the node test env can't
// load. taskTools only needs createToolDef to stamp the function name (as artifactTools.test).
vi.mock('../shared', () => ({
	createToolDef: (_schema: unknown, name: string, description: string) => ({
		type: 'function',
		function: { name, description, parameters: {} }
	})
}))

// The user-scoping subscription is BROWSER-gated; the node test env reports false.
vi.mock('esm-env', async (orig) => ({
	...(await orig<typeof import('esm-env')>()),
	BROWSER: true
}))

vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { userStore: writable(undefined) }
})
vi.mock('$lib/utils', () => ({ getLocalSetting: () => undefined, storeLocalSetting: () => {} }))

// sessionId has no default: passing undefined must stay undefined (the no-session case),
// not fall back to 's1' as a defaulted parameter would.
async function fresh(sessionId: string | undefined) {
	vi.resetModules()
	;(globalThis as any).indexedDB = new IDBFactory()
	;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
	const { SessionTasksStore } = await import('./tasksState.svelte')
	const { taskTools } = await import('./taskTools')
	const dbMod = await import('./tasksDB')
	const store = new SessionTasksStore()
	if (sessionId) await store.setSession(sessionId)
	const statuses: any[] = []
	const helpers = { tasks: store, sessionId }
	const byName = Object.fromEntries(taskTools.map((t) => [t.def.function.name, t])) as Record<
		string,
		Tool<{}>
	>
	const call = (name: string, args: any) =>
		byName[name].fn({
			args,
			workspace: 'w',
			helpers,
			toolId: 't',
			toolCallbacks: { setToolStatus: (_id: string, m: any) => statuses.push(m) } as any
		})
	return { call, store, dbMod, statuses }
}

const plan = [
	{ subject: 'Read the flow', description: 'read it' },
	{ subject: 'Wire the branch', description: 'wire it', activeForm: 'Wiring the branch' }
]

let ctx: Awaited<ReturnType<typeof fresh>>
beforeEach(async () => {
	ctx = await fresh('s1')
})

describe('task tools', () => {
	it('create_tasks persists the plan and returns ids plus a progress summary', async () => {
		const res = JSON.parse(await ctx.call('create_tasks', { tasks: plan }))
		expect(res).toMatchObject({ success: true, ids: [1, 2], summary: '0/2 done' })
		const stored = await ctx.dbMod.listTasksForSession('s1')
		expect(stored.map((t) => [t.seq, t.subject, t.status])).toEqual([
			[1, 'Read the flow', 'pending'],
			[2, 'Wire the branch', 'pending']
		])
	})

	it('update_task reports progress without echoing the task back', async () => {
		await ctx.call('create_tasks', { tasks: plan })
		const res = JSON.parse(await ctx.call('update_task', { id: 2, status: 'in_progress' }))
		// The model authored these; re-sending them every turn is the echo the ai-chat
		// skill forbids, so the payload stays {success, summary}.
		expect(Object.keys(res).sort()).toEqual(['success', 'summary'])
		expect(res.summary).toBe('0/2 done, now: Wire the branch')
	})

	it('update_task with status deleted removes the task and leaves the rest numbered', async () => {
		await ctx.call('create_tasks', { tasks: plan })
		const res = JSON.parse(await ctx.call('update_task', { id: 1, status: 'deleted' }))
		expect(res.success).toBe(true)
		const stored = await ctx.dbMod.listTasksForSession('s1')
		expect(stored.map((t) => t.seq)).toEqual([2])
	})

	it('update_task fails on an unknown id rather than creating one', async () => {
		await ctx.call('create_tasks', { tasks: plan })
		for (const args of [
			{ id: 99, status: 'completed' },
			{ id: 99, status: 'deleted' }
		]) {
			const res = JSON.parse(await ctx.call('update_task', args))
			expect(res.success).toBe(false)
			expect(res.error).toContain('99')
		}
		expect(await ctx.dbMod.listTasksForSession('s1')).toHaveLength(2)
	})

	it('list_tasks returns the plan with descriptions, for recovery after compaction', async () => {
		await ctx.call('create_tasks', { tasks: plan })
		const res = JSON.parse(await ctx.call('list_tasks', {}))
		expect(res).toEqual([
			{ id: 1, subject: 'Read the flow', description: 'read it', status: 'pending' },
			{ id: 2, subject: 'Wire the branch', description: 'wire it', status: 'pending' }
		])
	})

	it('rejects an empty plan and one past the per-call cap', async () => {
		const empty = JSON.parse(await ctx.call('create_tasks', { tasks: [] }))
		expect(empty.success).toBe(false)

		const tooMany = Array.from({ length: 21 }, (_, i) => ({ subject: `t${i}`, description: 'd' }))
		const capped = JSON.parse(await ctx.call('create_tasks', { tasks: tooMany }))
		expect(capped.success).toBe(false)
		expect(capped.error).toContain('21')
		expect(await ctx.dbMod.listTasksForSession('s1')).toHaveLength(0)
	})

	it('fails closed outside a session', async () => {
		const noSession = await fresh(undefined)
		for (const [name, args] of [
			['create_tasks', { tasks: plan }],
			['update_task', { id: 1, status: 'completed' }],
			['list_tasks', {}]
		] as const) {
			const res = JSON.parse(await noSession.call(name, args))
			expect(res.success).toBe(false)
			expect(res.error).toMatch(/only available inside an AI session/i)
		}
	})
})
