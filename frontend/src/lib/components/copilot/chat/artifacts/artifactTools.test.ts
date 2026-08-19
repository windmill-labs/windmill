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
	// planModeActive drives the store-side policy only: the gate that consults
	// refuseInPlanMode lives in the real ../shared, which is mocked out here.
	const call = (name: string, args: any, planModeActive = false) =>
		byName[name].fn({
			args,
			workspace: 'w',
			helpers,
			toolId: 't',
			toolCallbacks: {
				setToolStatus: (_id: string, m: any) => statuses.push(m),
				isPlanModeActive: () => planModeActive
			} as any
		})
	const refusal = (name: string, args: any) =>
		byName[name].refuseInPlanMode?.({ args, helpers: helpers as any })
	return { call, refusal, store, dbMod, statuses, opened }
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
		expect(list.find((x: any) => x.id === b.id)).toEqual({
			id: b.id,
			name: 'B',
			kind: 'md',
			version: 1
		})
		expect(list[0]).not.toHaveProperty('content')
	})

	it('creates the plan as a draft, and refuses a second one for the session', async () => {
		// This tool asks for no confirmation, so the model writing a plan document is not the
		// user agreeing to one. It holds the session's plan slot, but stays a draft until an
		// approval lands on it.
		const plan = JSON.parse(
			await ctx.call('create_artifact', { name: 'Ship it', content: '# Ship it', role: 'plan' })
		)
		expect(plan.id).toBe(ctx.dbMod.planArtifactId('s1'))
		expect(await ctx.dbMod.getArtifact(plan.id)).toMatchObject({
			role: 'plan',
			approvedVersion: undefined
		})

		// The slot is taken, and the refusal has to name what holds it — the model's next
		// move is to rewrite that document, which it cannot do without the id.
		const second = JSON.parse(
			await ctx.call('create_artifact', { name: 'Other', content: '# Other', role: 'plan' })
		)
		expect(second.success).toBe(false)
		expect(second.error).toContain(plan.id)
		const list = JSON.parse(await ctx.call('list_artifacts', {}))
		expect(list.filter((x: any) => x.role === 'plan').map((x: any) => x.id)).toEqual([plan.id])
	})

	it('reports which version of a plan the user approved, if any', async () => {
		// A plan is written when it is proposed, so one they refused stays on disk, and an
		// approved one keeps collecting versions afterwards. Without the pointer the model
		// reads whatever the document currently says as the plan they signed off.
		const plan = await ctx.store.create('s1', {
			name: 'Drafted',
			content: '# Drafted',
			role: 'plan',
			chatId: 'c1'
		})
		const drafted = JSON.parse(await ctx.call('list_artifacts', {})).find(
			(x: any) => x.id === plan.id
		)
		expect(drafted).toMatchObject({ role: 'plan', version: 1 })
		// Absent, not null: nothing here was ever approved.
		expect(drafted).not.toHaveProperty('approvedVersion')

		await ctx.store.update(plan.id, { approvedVersion: 1 })
		await ctx.store.update(plan.id, { content: '# Drafted, proposed anew' })
		const list = JSON.parse(await ctx.call('list_artifacts', {}))

		// Approved at v1, current text is v2: a proposal the user has not decided on.
		expect(list.find((x: any) => x.id === plan.id)).toMatchObject({
			role: 'plan',
			version: 2,
			approvedVersion: 1
		})
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
		const res = JSON.parse(
			await ctx.call('update_artifact', {
				id: a.id,
				content: 'v2',
				change_note: 'Reworded the intro'
			})
		)
		expect(res.success).toBe(true)
		expect((await ctx.dbMod.getArtifact(a.id))?.content).toBe('v2')
	})

	it('refuses in plan mode only what would write the plan, by either of its marks', async () => {
		const plan = await ctx.store.savePlan('s1', { name: 'Plan', content: 'v1', note: 'n' }, 'c1')
		const doc = JSON.parse(await ctx.call('create_artifact', { name: 'Doc', content: 'x' }))
		// A row claiming the role without the derived id: no writer here mints one, and the
		// refusal must not depend on that staying true.
		await ctx.dbMod.putArtifact({
			id: 'odd',
			sessionId: 's1',
			kind: 'md',
			role: 'plan',
			name: 'Odd',
			content: 'x',
			createdAt: 0,
			updatedAt: 0,
			version: 1
		})
		await ctx.store.setSession(undefined)
		await ctx.store.setSession('s1')

		expect(ctx.refusal('create_artifact', { name: 'P', content: 'x', role: 'plan' })).toBeDefined()
		expect(ctx.refusal('create_artifact', { name: 'D', content: 'x' })).toBeUndefined()
		expect(ctx.refusal('update_artifact', { id: plan.id, content: 'x' })).toBeDefined()
		expect(ctx.refusal('update_artifact', { id: 'odd', content: 'x' })).toBeDefined()
		expect(ctx.refusal('update_artifact', { id: doc.id, content: 'x' })).toBeUndefined()
	})

	it('refuses a plan write the posture only turned down mid-flight', async () => {
		const plan = await ctx.store.savePlan('s1', { name: 'Plan', content: 'v1', note: 'n' }, 'c1')

		const updated = JSON.parse(
			await ctx.call(
				'update_artifact',
				{ id: plan.id, content: 'rewritten', change_note: 'x' },
				true
			)
		)
		expect(updated.success).toBe(false)
		expect(updated.error).toMatch(/not writable in plan mode/)
		expect((await ctx.dbMod.getArtifact(plan.id))?.content).toBe('v1')
		expect(ctx.statuses.at(-1)).toMatchObject({ blockedByPlanMode: true })

		const created = JSON.parse(
			await ctx.call('create_artifact', { name: 'P', content: 'x', role: 'plan' }, true)
		)
		expect(created.success).toBe(false)
		expect(created.error).toMatch(/not writable in plan mode/)
	})

	it('update_artifact reports a missing id', async () => {
		const res = JSON.parse(
			await ctx.call('update_artifact', { id: 'nope', content: 'x', change_note: 'n/a' })
		)
		expect(res.success).toBe(false)
		expect(res.error).toMatch(/No artifact/)
	})

	it('read_artifact reports a failed version read as retryable, not as a missing version', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		await ctx.call('update_artifact', { id: a.id, content: 'v2', change_note: 'second' })
		vi.spyOn(ctx.dbMod, 'getArtifactVersion').mockRejectedValue(new Error('read failed'))

		// Naming it absent would send the model to list_artifact_versions and have it conclude
		// the version is gone, when nothing has been read at all.
		const res = JSON.parse(await ctx.call('read_artifact', { id: a.id, version: 1 }))
		expect(res.success).toBe(false)
		expect(res.error).toMatch(/unavailable/)
		expect(res.error).not.toMatch(/no version/)
	})

	it('read_artifact reports a missing id', async () => {
		const res = JSON.parse(await ctx.call('read_artifact', { id: 'nope' }))
		expect(res.success).toBe(false)
	})

	it('exposes the version history and reads an earlier version by number', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		await ctx.call('update_artifact', {
			id: a.id,
			content: 'v2',
			change_note: 'Reworded the intro'
		})

		const versions = JSON.parse(await ctx.call('list_artifact_versions', { id: a.id }))
		expect(versions.map((v: any) => [v.version, v.current, v.note])).toEqual([
			[2, true, 'Reworded the intro'],
			// A first version has no note — the picker labels it itself.
			[1, false, undefined]
		])
		expect(JSON.parse(await ctx.call('read_artifact', { id: a.id, version: 1 })).content).toBe('v1')
		// Omitting the version, and naming the current one, both read the live content.
		expect(JSON.parse(await ctx.call('read_artifact', { id: a.id })).content).toBe('v2')
		expect(JSON.parse(await ctx.call('read_artifact', { id: a.id, version: 2 })).content).toBe('v2')
	})

	it('stores an overlong change note truncated rather than failing the update', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		const res = JSON.parse(
			await ctx.call('update_artifact', { id: a.id, content: 'v2', change_note: 'x'.repeat(300) })
		)
		// The content edit must land: the note is a label, not a reason to reject the write.
		expect(res.success).toBe(true)
		expect((await ctx.dbMod.getArtifact(a.id))?.content).toBe('v2')
		const [latest] = await ctx.store.listVersions(a.id)
		expect(latest.note).toHaveLength(120)
	})

	it('stores a blank change note as absent so the picker can label it', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		// Whitespace-only survives zod's `z.string()`; "" is not nullish, so it would slip
		// past the picker's fallback and render a row with no label at all.
		await ctx.call('update_artifact', { id: a.id, content: 'v2', change_note: '   ' })

		const [latest] = await ctx.store.listVersions(a.id)
		expect(latest.note).toBeUndefined()
	})

	it('read_artifact reports a version that was never saved or has been pruned', async () => {
		const a = JSON.parse(await ctx.call('create_artifact', { name: 'A', content: 'v1' }))
		const res = JSON.parse(await ctx.call('read_artifact', { id: a.id, version: 7 }))
		expect(res.success).toBe(false)
		expect(res.error).toMatch(/list_artifact_versions/)
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
		for (const name of [
			'create_artifact',
			'list_artifacts',
			'update_artifact',
			'read_artifact',
			'list_artifact_versions'
		]) {
			const res = JSON.parse(
				await noSession.call(name, { id: 'x', name: 'A', content: 'a', change_note: 'n/a' })
			)
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
		const updated = JSON.parse(
			await ctx.call('update_artifact', { id: 'other', content: 'x', change_note: 'n/a' })
		)
		expect(updated.success).toBe(false)
		// The other session's content is untouched.
		expect((await ctx.dbMod.getArtifact('other'))?.content).toBe('secret')
	})
})
