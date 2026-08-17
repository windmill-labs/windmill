import { beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { planFirst, SessionArtifactsStore } from './artifactsState.svelte'
import * as db from './artifactsDB'

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

// The DB module memoises its handle at module scope. A fresh IDBFactory per test only
// isolates data once the handle is reset, so reset modules and re-import both together.
// The DB is namespaced by email, so seed a user.
async function fresh() {
	vi.resetModules()
	;(globalThis as any).indexedDB = new IDBFactory()
	;(await import('$lib/stores')).userStore.set({ email: 'a@x.com' } as never)
	const dbMod = await import('./artifactsDB')
	// The re-imported module, not the file's static import: resetModules gives the store a fresh
	// copy, and an error class from the stale one would never match what it throws.
	const stateMod = await import('./artifactsState.svelte')
	return { dbMod, stateMod, store: new stateMod.SessionArtifactsStore() }
}

let store: SessionArtifactsStore
let dbMod: typeof db
let stateMod: typeof import('./artifactsState.svelte')
beforeEach(async () => {
	;({ store, dbMod, stateMod } = await fresh())
})

describe('SessionArtifactsStore', () => {
	it('loads the current session, newest-updated first', async () => {
		await dbMod.putArtifact(mk({ id: 'old', sessionId: 's1', updatedAt: 1 }))
		await dbMod.putArtifact(mk({ id: 'new', sessionId: 's1', updatedAt: 2 }))
		await dbMod.putArtifact(mk({ id: 'other', sessionId: 's2', updatedAt: 3 }))

		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.id)).toEqual(['new', 'old'])
		expect(store.loading).toBe(false)
	})

	it('empties the list for an undefined session id', async () => {
		await dbMod.putArtifact(mk({ id: 'a', sessionId: 's1' }))
		await store.setSession('s1')
		expect(store.artifacts).toHaveLength(1)

		await store.setSession(undefined)
		expect(store.artifacts).toEqual([])
	})

	it('a later setSession wins over an earlier in-flight load', async () => {
		await dbMod.putArtifact(mk({ id: 'a', sessionId: 's1' }))
		await dbMod.putArtifact(mk({ id: 'b', sessionId: 's2' }))

		// Start both loads without awaiting the first; the last-started must win.
		const first = store.setSession('s1')
		const second = store.setSession('s2')
		await Promise.all([first, second])
		expect(store.artifacts.map((a) => a.id)).toEqual(['b'])
	})

	it('a create is not clobbered by an in-flight load with a stale snapshot', async () => {
		// Hold the load open with a stale (empty) snapshot until after the create lands.
		let releaseLoad!: (items: db.PersistedArtifact[]) => void
		const held = new Promise<db.PersistedArtifact[]>((r) => (releaseLoad = r))
		const spy = vi.spyOn(dbMod, 'listArtifactsForSession').mockReturnValueOnce(held)

		const loading = store.setSession('s1') // #load starts, hangs on `held`
		const created = await store.create('s1', { name: 'X', content: 'x' })
		releaseLoad([]) // the load resolves late, snapshot predates the create
		await loading
		spy.mockRestore()

		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// The superseded load early-returns; the create must have cleared `loading`.
		expect(store.loading).toBe(false)
	})

	it('create persists and prepends when the session is loaded', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: '# hi' })

		expect(created.kind).toBe('md')
		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// Persisted: switching away and back reloads it from the DB.
		await store.setSession('other')
		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.name)).toEqual(['Plan'])
	})

	it('a same-session resync keeps an artifact whose persist failed', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })
		// Simulate a persist that never landed: drop it from the DB, keep it in memory.
		await dbMod.deleteArtifact(created.id)
		// A routine resync (chat rotation / global-mode reconfig) must not reload it away.
		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
	})

	it('create stamps the provenance chatId when supplied', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'x', chatId: 'c9' })
		expect(created.chatId).toBe('c9')
	})

	it('create persists to another session without touching the loaded list', async () => {
		await store.setSession('s1')
		const created = await store.create('s2', { name: 'Elsewhere', content: 'x' })
		expect(store.artifacts).toEqual([])
		expect((await dbMod.listArtifactsForSession('s2')).map((a) => a.id)).toEqual([created.id])
	})

	it('update merges changes, bumps updatedAt, and persists', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'v1' })

		const updated = await store.update(created.id, { content: 'v2' })
		expect(updated?.content).toBe('v2')
		expect(updated?.name).toBe('Plan')
		expect(updated!.updatedAt).toBeGreaterThanOrEqual(created.updatedAt)
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v2')
	})

	it('update falls back to the DB when the target is not in the loaded list', async () => {
		const created = await store.create('s2', { name: 'Off', content: 'v1' })
		await store.setSession('s1') // s2 is not loaded

		const updated = await store.update(created.id, { name: 'Renamed' })
		expect(updated?.name).toBe('Renamed')
		expect(updated?.content).toBe('v1')
	})

	it('update returns undefined for an unknown id', async () => {
		await store.setSession('s1')
		expect(await store.update('nope', { content: 'x' })).toBeUndefined()
	})

	it('gives two tabs editing at once distinct versions, keeping both snapshots', async () => {
		// Without the transactional read both tabs stamp the same next version, and one edit
		// and its snapshot vanish under the other.
		const other = new (await import('./artifactsState.svelte')).SessionArtifactsStore()
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Doc', content: 'v1' })
		await other.setSession('s1')

		await Promise.all([
			store.update(created.id, { content: 'from A', note: 'A' }),
			other.update(created.id, { content: 'from B', note: 'B' })
		])

		expect((await dbMod.listArtifactVersions(created.id)).map((v) => v.version)).toEqual([3, 2, 1])
		expect((await dbMod.getArtifact(created.id))?.version).toBe(3)
	})

	it('keeps an edit the store refused when the next update lands', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Doc', content: 'v1' })

		// Refused, so v2 lives only in memory while the stored row stays behind at v1.
		const quiet = vi.spyOn(console, 'error').mockImplementation(() => {})
		const put = vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementationOnce(() => {
			throw new DOMException('quota', 'QuotaExceededError')
		})
		await store.update(created.id, { content: 'v2' })
		put.mockRestore()
		quiet.mockRestore()
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v1')

		// Computed from the stored row, this rename would put v1's text back under the new name.
		expect(await store.update(created.id, { name: 'Renamed' })).toMatchObject({
			name: 'Renamed',
			content: 'v2'
		})
	})

	it('creates and then revises an artifact when there is no store to write to', async () => {
		// No scoped user, so the database never opens. `create` hands the id out either way, so
		// the document it named has to stay revisable rather than come back as an unknown one.
		;(await import('$lib/stores')).userStore.set(undefined as never)
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })

		expect((await store.update(created.id, { content: 'y' }))?.content).toBe('y')
		expect(store.artifacts.map((a) => a.content)).toEqual(['y'])
	})

	it('get resolves from the in-memory list even when the DB lacks the record', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })
		// Simulate a persist that never landed: drop it from the DB, keep it in memory.
		await dbMod.deleteArtifact(created.id)
		expect((await store.get(created.id))?.id).toBe(created.id)
	})

	it('get falls back to the DB for an artifact outside the loaded list', async () => {
		const created = await store.create('s2', { name: 'B', content: 'y' })
		await store.setSession('s1') // s2 not loaded
		expect((await store.get(created.id))?.id).toBe(created.id)
		expect(await store.get('nope')).toBeUndefined()
	})

	it('listForSession returns the in-memory list for the loaded session, even when the DB lacks it', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'A', content: 'x' })
		await dbMod.deleteArtifact(created.id) // persist "failed": gone from DB, kept in memory
		expect((await store.listForSession('s1')).map((a) => a.id)).toEqual([created.id])
	})

	it('listForSession reads the DB for a session that is not loaded', async () => {
		await store.create('s2', { name: 'B', content: 'y' })
		await store.setSession('s1') // s2 not loaded
		expect((await store.listForSession('s2')).map((a) => a.name)).toEqual(['B'])
		expect(await store.listForSession('s1')).toEqual([])
	})

	it('update refuses an artifact from a different session when scoped by sessionId', async () => {
		const created = await store.create('s2', { name: 'Off', content: 'v1' })
		expect(await store.update(created.id, { content: 'v2' }, { sessionId: 's1' })).toBeUndefined()
		// Unchanged in the DB.
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v1')
		// Correct scope still updates.
		expect(await store.update(created.id, { content: 'v2' }, { sessionId: 's2' })).toBeDefined()
	})

	it('update distinguishes an omitted field from an explicit empty string', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'body' })

		// Omitting content leaves it untouched...
		expect((await store.update(created.id, { name: 'Renamed' }))?.content).toBe('body')
		// ...but an explicit empty string blanks it.
		expect((await store.update(created.id, { content: '' }))?.content).toBe('')
	})

	it('snapshots a version per content change, and none for a rename', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'v1' })

		await store.update(created.id, { content: 'v2', note: 'Added a rollback step' })
		// Neither a rename nor a rewrite to the identical content is a new version.
		await store.update(created.id, { name: 'Renamed', note: 'ignored' })
		await store.update(created.id, { content: 'v2', note: 'ignored' })

		const versions = await store.listVersions(created.id)
		expect(versions.map((v) => [v.version, v.content, v.note])).toEqual([
			[2, 'v2', 'Added a rollback step'],
			[1, 'v1', undefined]
		])
		expect((await store.get(created.id))?.version).toBe(2)
		expect((await store.getVersion(created.id, 1))?.content).toBe('v1')
	})

	it('persists a row and the snapshots it produced in one write', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Doc', content: 'c1' })

		const spy = vi.spyOn(dbMod, 'mutateArtifact')
		await store.update(created.id, { content: 'c2', note: 'second' })

		// Split into two writes, a stamped version can outlive the snapshot that failed to
		// land beside it. That state still reads as complete, because listVersions
		// synthesizes the current version from the row — right up until the next edit
		// overwrites the row, which is the only copy of that content left.
		expect(spy).toHaveBeenCalledTimes(1)
		spy.mockRestore()
		expect((await store.listVersions(created.id)).map((v) => v.version)).toEqual([2, 1])

		const row = await dbMod.getArtifact(created.id)
		const stored = await dbMod.listArtifactVersions(created.id)
		expect(stored.some((v) => v.version === row!.version)).toBe(true)
	})

	it('serves the live version from memory when the store is unreadable, but not an older one', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'v1' })
		await store.update(created.id, { content: 'v2', note: 'second' })
		vi.spyOn(dbMod, 'getArtifactVersion').mockRejectedValue(new Error('read failed'))

		expect((await store.getVersion(created.id, 2))?.content).toBe('v2')
		await expect(store.getVersion(created.id, 1)).rejects.toThrow('read failed')
	})

	it('keeps a legacy v1 when a rename lands before the first content edit', async () => {
		await dbMod.putArtifact(mk({ id: 'legacy', content: 'original' }))
		await store.setSession('s1')

		// The rename stamps `version`, after which nothing else would recognise this as a
		// pre-history artifact — so its v1 has to be captured here or it is lost for good.
		await store.update('legacy', { name: 'Renamed' })
		await store.update('legacy', { content: 'edited', note: 'Rewrote it' })

		expect((await store.listVersions('legacy')).map((v) => [v.version, v.content])).toEqual([
			[2, 'edited'],
			[1, 'original']
		])
	})

	it('reads an artifact stored before history existed as its own version 1', async () => {
		await dbMod.putArtifact(mk({ id: 'legacy', content: 'only' }))
		await store.setSession('s1')

		expect(await store.listVersions('legacy')).toMatchObject([{ version: 1, content: 'only' }])
		expect((await store.getVersion('legacy', 1))?.content).toBe('only')
		// Its first edit still lands as v2, so version numbers stay monotonic.
		await store.update('legacy', { content: 'edited' })
		expect((await store.listVersions('legacy')).map((v) => v.version)).toEqual([2, 1])
	})

	it('a plan another tab created joins the loaded list when this one revises it', async () => {
		await store.setSession('s1')
		// Written straight to the DB under the id the session derives: this store loaded s1
		// before the plan existed, which is the cross-tab case.
		await dbMod.putArtifact({
			id: dbMod.planArtifactId('s1'),
			sessionId: 's1',
			kind: 'md',
			role: 'plan',
			name: 'Theirs',
			content: 'v1',
			createdAt: 1,
			updatedAt: 1,
			version: 1
		})

		await store.savePlan('s1', { name: 'Theirs', content: 'v2', note: 'revised' }, undefined)

		// Persisted but absent from here would leave it out of the preview, the transcript's
		// plan card and list_artifacts until a reload.
		expect(store.artifacts.map((a) => a.id)).toEqual([dbMod.planArtifactId('s1')])
		expect(store.artifacts[0].content).toBe('v2')
	})

	it('refuses a plan, and an approval, the store would not keep', async () => {
		await store.setSession('s1')
		const quiet = vi.spyOn(console, 'error').mockImplementation(() => {})
		const refuseOnce = () =>
			vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementationOnce(() => {
				throw new DOMException('quota', 'QuotaExceededError')
			})

		// An ordinary artifact degrades unpersisted; a plan cannot, because approving one that
		// is gone on reload leaves the user agreeing to a document nothing can show them.
		let put = refuseOnce()
		await expect(
			store.savePlan('s1', { name: 'Plan', content: '# p', note: 'first' }, undefined)
		).rejects.toThrow(/could not be saved/)
		put.mockRestore()

		const plan = await store.savePlan(
			's1',
			{ name: 'Plan', content: '# p', note: 'first' },
			undefined
		)
		put = refuseOnce()
		expect(await store.approve(plan.id, 1)).toBe(false)
		put.mockRestore()
		// And the refusal is not merely reported: an approval reflected in memory anyway would
		// show the `plan` pill, and tell the model the user signed off, until the next reload.
		expect(store.artifacts.find((a) => a.id === plan.id)?.approvedVersion).toBeUndefined()
		expect((await store.get(plan.id))?.approvedVersion).toBeUndefined()

		expect(await store.approve(plan.id, 1)).toBe(true)
		expect(store.artifacts.find((a) => a.id === plan.id)?.approvedVersion).toBe(1)
		quiet.mockRestore()
	})

	it('holds one plan per session, and frees the slot when it is deleted', async () => {
		await store.setSession('s1')
		const plan = await store.create('s1', { name: 'Plan', content: 'x', role: 'plan' })
		await expect(store.create('s1', { name: 'Other', content: 'y', role: 'plan' })).rejects.toThrow(
			/already has a plan/
		)
		// Another session's slot is its own.
		await expect(
			store.create('s2', { name: 'Elsewhere', content: 'z', role: 'plan' })
		).resolves.toBeDefined()

		await store.remove(plan.id)
		await expect(store.create('s1', { name: 'Next', content: 'w', role: 'plan' })).resolves.toEqual(
			expect.objectContaining({ role: 'plan' })
		)
	})

	it('gives two tabs proposing at once distinct versions, keeping both snapshots', async () => {
		// The hazard the transaction exists for: read outside it and both tabs stamp the same
		// next version, so one proposal and its snapshot vanish under the other.
		const other = new (await import('./artifactsState.svelte')).SessionArtifactsStore()
		await store.setSession('s1')
		await store.savePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined)

		await Promise.all([
			store.savePlan('s1', { name: 'Plan', content: 'from A', note: 'A' }, undefined),
			other.savePlan('s1', { name: 'Plan', content: 'from B', note: 'B' }, undefined)
		])

		const versions = (await store.listVersions(dbMod.planArtifactId('s1'))).map((v) => v.version)
		expect(versions).toEqual([3, 2, 1])
		expect((await dbMod.getArtifact(dbMod.planArtifactId('s1')))?.version).toBe(3)
	})

	it('an approval racing a newer proposal moves the pointer without reverting content', async () => {
		// approve() must patch, never rewrite: an approval computed while another tab was
		// revising would otherwise carry this tab's older text back over the newer one.
		const other = new (await import('./artifactsState.svelte')).SessionArtifactsStore()
		await store.setSession('s1')
		const plan = await store.savePlan(
			's1',
			{ name: 'Plan', content: 'v1', note: 'first' },
			undefined
		)

		await other.savePlan(
			's1',
			{ name: 'Plan', content: 'v2 from the other tab', note: 'B' },
			undefined
		)
		await store.approve(plan.id, 1)

		const row = await dbMod.getArtifact(plan.id)
		expect(row).toMatchObject({ content: 'v2 from the other tab', version: 2, approvedVersion: 1 })
	})

	it('leaves the plan alone for a caller whose policy refuses it, and only that caller', async () => {
		await store.setSession('s1')
		const plan = await store.savePlan(
			's1',
			{ name: 'Plan', content: 'v1', note: 'first' },
			undefined
		)
		const doc = await store.create('s1', { name: 'Doc', content: 'x' })
		const no = { canWritePlan: () => false }

		await expect(store.update(plan.id, { content: 'rewritten' }, no)).rejects.toThrow(
			stateMod.PlanWriteRefusedError
		)
		await expect(
			store.create('s2', { name: 'Plan', content: 'x', role: 'plan' }, no)
		).rejects.toThrow(stateMod.PlanWriteRefusedError)
		// The policy is about the plan, not about writing: everything else is untouched by it.
		await expect(store.update(doc.id, { content: 'y' }, no)).resolves.toMatchObject({
			content: 'y'
		})
		// And a caller that states no policy keeps every posture's plan writes exactly as they were.
		await expect(store.update(plan.id, { content: 'v2' })).resolves.toMatchObject({ version: 2 })
		expect((await dbMod.getArtifact(plan.id))?.content).toBe('v2')
	})

	it('refuses an approval naming no readable version, or no plan at all', async () => {
		await store.setSession('s1')
		const plan = await store.savePlan(
			's1',
			{ name: 'Plan', content: 'v1', note: 'first' },
			undefined
		)
		const doc = await store.create('s1', { name: 'Doc', content: 'x' })

		await expect(store.approve(plan.id, 2)).resolves.toBe(false)
		await expect(store.approve(plan.id, 0)).resolves.toBe(false)
		await expect(store.approve(plan.id, 1.5)).resolves.toBe(false)
		await expect(store.approve(plan.id, NaN)).resolves.toBe(false)
		await expect(store.approve(doc.id, 1)).resolves.toBe(false)
		expect((await dbMod.getArtifact(plan.id))?.approvedVersion).toBeUndefined()

		await expect(store.approve(plan.id, 1)).resolves.toBe(true)
	})

	it('approves a retained older version after history has moved on', async () => {
		// The approval a card proposed lands late, and the versions between it and the head are
		// still stored — the pointer belongs on the version the user read, not on the newest.
		await store.setSession('s1')
		const plan = await store.savePlan(
			's1',
			{ name: 'Plan', content: 'v1', note: 'first' },
			undefined
		)
		await store.savePlan('s1', { name: 'Plan', content: 'v2', note: 'second' }, undefined)
		await store.savePlan('s1', { name: 'Plan', content: 'v3', note: 'third' }, undefined)

		await expect(store.approve(plan.id, 1)).resolves.toBe(true)
		expect(await dbMod.getArtifact(plan.id)).toMatchObject({ version: 3, approvedVersion: 1 })
	})

	it('does not move the approval when a write produces no new version', async () => {
		// A plan approved at v1 and revised into a proposal the user turned down. Renaming it,
		// or rewriting it with the text already there, adds no version — so there is nothing
		// for the approval to move onto, and the refused text must stay refused.
		await store.setSession('s1')
		const plan = await store.create('s1', { name: 'Plan', content: 'v1', role: 'plan' })
		await store.update(plan.id, { approvedVersion: 1 })
		const refused = await store.update(plan.id, { content: 'v2 the user rejected' })
		expect(refused).toMatchObject({ version: 2, approvedVersion: 1 })

		const renamed = await store.update(plan.id, {
			name: 'Plan, renamed',
			content: 'v2 the user rejected',
			keepApproved: true
		})
		expect(renamed).toMatchObject({ version: 2, approvedVersion: 1 })

		// A write that does add a version still carries it: an edit outside plan mode is one
		// the user's posture already trusts.
		const revised = await store.update(plan.id, { content: 'v3', keepApproved: true })
		expect(revised).toMatchObject({ version: 3, approvedVersion: 3 })
	})

	it('remove deletes from the DB and the loaded list', async () => {
		await store.setSession('s1')
		const created = await store.create('s1', { name: 'Plan', content: 'x' })

		await store.remove(created.id)
		expect(store.artifacts).toEqual([])
		expect(await dbMod.getArtifact(created.id)).toBeUndefined()
	})
})

function mk(over: Partial<db.PersistedArtifact> = {}): db.PersistedArtifact {
	return {
		id: 'a1',
		sessionId: 's1',
		chatId: 'c1',
		kind: 'md',
		name: 'Doc',
		content: '# hi',
		createdAt: 0,
		updatedAt: 0,
		...over
	}
}

describe('planFirst', () => {
	it('pins plans above artifacts updated more recently', () => {
		// Store order is newest-first; a run that writes a CSV after the plan is
		// approved would otherwise bury the plan the user keeps coming back to.
		const csv = mk({ id: 'csv', name: 'runs.csv', updatedAt: 20 })
		const plan = mk({ id: 'plan', name: 'Add retries', role: 'plan', updatedAt: 10 })
		expect(planFirst([csv, plan]).map((a) => a.id)).toEqual(['plan', 'csv'])
	})

	it('keeps each group newest-first, and several plans together', () => {
		const newPlan = mk({ id: 'p2', role: 'plan', updatedAt: 30 })
		const doc = mk({ id: 'doc', updatedAt: 20 })
		const oldPlan = mk({ id: 'p1', role: 'plan', updatedAt: 10 })
		expect(planFirst([newPlan, doc, oldPlan]).map((a) => a.id)).toEqual(['p2', 'p1', 'doc'])
	})
})
