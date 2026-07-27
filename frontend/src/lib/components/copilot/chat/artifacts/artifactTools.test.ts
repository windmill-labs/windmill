import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import type { Tool } from '../shared'

// The real ../shared pulls the whole component/monaco graph, which the node test env can't
// load. artifactTools only needs createToolDef to stamp the function name (as datatableTools.test).
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

// Stub $lib/stores + $lib/utils (userScopedStorage's only deps here) to keep their heavy
// svelte/app-store graphs out of the per-test cold transform.
vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { userStore: writable(undefined) }
})
vi.mock('$lib/utils', () => ({ getLocalSetting: () => undefined, storeLocalSetting: () => {} }))

// Reset the memoised DB handle and install a fresh store per test (see artifactsDB tests).
// sessionId has no default: passing undefined must stay undefined (the no-session case), not
// fall back to 's1' as a defaulted parameter would. The DB is namespaced by email, so seed a user.
async function fresh(sessionId: string | undefined) {
	vi.resetModules()
	;(globalThis as any).indexedDB = new IDBFactory()
	;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
	const { SessionArtifactsStore } = await import('./artifactsState.svelte')
	const { artifactTools } = await import('./artifactTools')
	const dbMod = await import('./artifactsDB')
	const store = new SessionArtifactsStore()
	const statuses: any[] = []
	const opened: Array<[string, string]> = []
	const helpers = {
		artifacts: store,
		sessionId,
		getChatId: () => 'c1',
		openArtifact: (id: string, name: string) => opened.push([id, name])
	}
	const byName = Object.fromEntries(artifactTools.map((t) => [t.def.function.name, t])) as Record<
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
	return { call, store, dbMod, statuses, opened }
}

let ctx: Awaited<ReturnType<typeof fresh>>
beforeEach(async () => {
	ctx = await fresh('s1')
})

describe('artifact tools', () => {
	it('create_artifact persists (session-scoped, chat provenance) and opens the preview', async () => {
		const res = JSON.parse(await ctx.call('create_artifact', { name: 'Plan', content: '# hi' }))
		expect(res.success).toBe(true)
		expect(res.name).toBe('Plan')
		const stored = await ctx.dbMod.getArtifact(res.id)
		expect(stored).toMatchObject({
			name: 'Plan',
			content: '# hi',
			kind: 'md',
			sessionId: 's1',
			chatId: 'c1'
		})
		expect(ctx.opened).toEqual([[res.id, 'Plan']])
	})

	it('list_artifacts returns id/name/kind without content', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'a' }))
		const b = JSON.parse(await ctx.call('create_artifact', { name: 'B', content: 'b' }))
		const list = JSON.parse(await ctx.call('list_artifacts', {}))
		expect(list.map((x: any) => x.id).sort()).toEqual([a.id, b.id].sort())
		expect(list.find((x: any) => x.id === b.id)).toEqual({ id: b.id, name: 'B', kind: 'md' })
		expect(list[0]).not.toHaveProperty('content')
	})

	it('read_artifact returns the full content', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'body' }))
		const read = JSON.parse(await ctx.call('read_artifact', { id: a.id }))
		expect(read).toMatchObject({ id: a.id, name: 'A', kind: 'md', content: 'body' })
	})

	it('list_artifacts still returns a create whose persist was swallowed', async () => {
		await ctx.store.setSession('s1') // load the session so create reflects in the in-memory list
		const created = JSON.parse(await ctx.call('create_artifact', { name: 'Ghost', content: 'x' }))
		await ctx.dbMod.deleteArtifact(created.id) // mimic a quota-swallowed persist: gone from the DB
		expect(await ctx.dbMod.getArtifact(created.id)).toBeUndefined()
		// read and list both stay consistent via the in-memory fallback.
		const list = JSON.parse(await ctx.call('list_artifacts', {}))
		expect(list.map((x: any) => x.id)).toContain(created.id)
	})

	it('update_artifact overwrites content and persists', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		const res = JSON.parse(await ctx.call('update_artifact', { id: a.id, content: 'v2' }))
		expect(res.success).toBe(true)
		expect((await ctx.dbMod.getArtifact(a.id))?.content).toBe('v2')
	})

	it('update_artifact reports a missing id', async () => {
		const res = JSON.parse(await ctx.call('update_artifact', { id: 'nope', content: 'x' }))
		expect(res.success).toBe(false)
		expect(res.error).toMatch(/No artifact/)
	})

	it('read_artifact reports a missing id', async () => {
		const res = JSON.parse(await ctx.call('read_artifact', { id: 'nope' }))
		expect(res.success).toBe(false)
	})

	it('rejects content over the size cap without persisting', async () => {
		const huge = 'x'.repeat(256 * 1024 + 1)
		const res = JSON.parse(await ctx.call('create_artifact', { name: 'Big', content: huge }))
		expect(res.success).toBe(false)
		expect(res.error).toMatch(/too large/)
		expect(await ctx.dbMod.listArtifactsForSession('s1')).toEqual([])
	})

	it('reports unavailable when there is no session', async () => {
		const noSession = await fresh(undefined)
		for (const name of ['create_artifact', 'list_artifacts', 'update_artifact', 'read_artifact']) {
			const res = JSON.parse(await noSession.call(name, { id: 'x', name: 'A', content: 'a' }))
			expect(res.success).toBe(false)
			expect(res.error).toMatch(/inside an AI session/)
		}
	})

	it('update_artifact and read_artifact ignore ids from another session', async () => {
		// Belongs to s2; the tools resolve session 's1'.
		await ctx.dbMod.putArtifact({
			id: 'other',
			sessionId: 's2',
			chatId: 'c2',
			kind: 'md',
			name: 'Other',
			content: 'secret',
			createdAt: 0,
			updatedAt: 0
		})
		const read = JSON.parse(await ctx.call('read_artifact', { id: 'other' }))
		expect(read.success).toBe(false)
		const updated = JSON.parse(await ctx.call('update_artifact', { id: 'other', content: 'x' }))
		expect(updated.success).toBe(false)
		// The other session's content is untouched.
		expect((await ctx.dbMod.getArtifact('other'))?.content).toBe('secret')
	})
})
