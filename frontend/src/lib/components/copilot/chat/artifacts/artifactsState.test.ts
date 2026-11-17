import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { planFirst, type SessionArtifactsStore } from './artifactsState.svelte'
import type * as db from './artifactsDB'
import type { WriteResult } from './artifactWriter'

// The user-scoping subscription is BROWSER-gated; the node test env reports false.
vi.mock('esm-env', async (orig) => ({
	...(await orig<typeof import('esm-env')>()),
	BROWSER: true
}))

// Stub $lib/stores + $lib/utils (userScopedStorage's only deps here) to keep their heavy
// svelte/app-store graphs out of the per-test cold transform.
vi.mock('$lib/stores', async () => {
	const { writable } = await import('svelte/store')
	return { userStore: writable(undefined), userWorkspaces: writable([]) }
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
	const stateMod = await import('./artifactsState.svelte')
	return { dbMod, stateMod, store: new stateMod.SessionArtifactsStore() }
}

let store: SessionArtifactsStore
let dbMod: typeof db
let stateMod: typeof import('./artifactsState.svelte')
beforeEach(async () => {
	;({ store, dbMod, stateMod } = await fresh())
})
afterEach(() => {
	vi.restoreAllMocks()
})

function saved(result: WriteResult): db.PersistedArtifact {
	if (result.outcome !== 'saved') throw new Error(`not saved: ${result.outcome}`)
	return result.artifact
}

/** Fail this browser's next write the way a full storage does. */
function refuseNextWrite() {
	vi.spyOn(console, 'error').mockImplementation(() => {})
	vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementationOnce(() => {
		throw new DOMException('quota', 'QuotaExceededError')
	})
}

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
		vi.spyOn(dbMod, 'listArtifactsForSession').mockReturnValueOnce(held)

		const loading = store.setSession('s1') // #load starts, hangs on `held`
		const created = saved(await store.create('s1', { name: 'X', content: 'x' }))
		releaseLoad([]) // the load resolves late, snapshot predates the create
		await loading

		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// The superseded load early-returns; the create must have cleared `loading`.
		expect(store.loading).toBe(false)
	})

	it('create persists and prepends when the session is loaded', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: '# hi' }))

		expect(created.kind).toBe('md')
		expect(store.artifacts.map((a) => a.id)).toEqual([created.id])
		// Persisted: switching away and back reloads it from the DB.
		await store.setSession('other')
		await store.setSession('s1')
		expect(store.artifacts.map((a) => a.name)).toEqual(['Plan'])
	})

	it('create stamps the provenance chatId when supplied', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: 'x', chatId: 'c9' }))
		expect(created.chatId).toBe('c9')
	})

	it('create persists to another session without touching the loaded list', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s2', { name: 'Elsewhere', content: 'x' }))
		expect(store.artifacts).toEqual([])
		expect((await dbMod.listArtifactsForSession('s2')).map((a) => a.id)).toEqual([created.id])
	})

	it('stores an edit as the next version, keeping what it does not change', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: 'v1' }))

		const result = await store.edit(created.id, { content: 'v2' })
		expect(result).toMatchObject({ outcome: 'saved', version: 2 })
		const edited = saved(result)
		expect(edited).toMatchObject({ name: 'Plan', content: 'v2' })
		expect(edited.updatedAt).toBeGreaterThanOrEqual(created.updatedAt)
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v2')
	})

	it('edits an artifact of a session that is not loaded', async () => {
		const created = saved(await store.create('s2', { name: 'Off', content: 'v1' }))
		await store.setSession('s1')

		expect(saved(await store.edit(created.id, { name: 'Renamed' }))).toMatchObject({
			name: 'Renamed',
			content: 'v1'
		})
	})

	it('answers missing for an unknown id', async () => {
		await store.setSession('s1')
		expect(await store.edit('nope', { content: 'x' })).toEqual({ outcome: 'missing' })
	})

	it('stores only the first of two edits two tabs make at once on one version', async () => {
		// The second, stored over the first, would replace a version it never saw.
		const other = new stateMod.SessionArtifactsStore()
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Doc', content: 'v1' }))
		await other.setSession('s1')

		const results = await Promise.all([
			store.edit(created.id, { content: 'from A', note: 'A' }),
			other.edit(created.id, { content: 'from B', note: 'B' })
		])

		expect(results.map((r) => r.outcome).sort()).toEqual(['saved', 'stale'])
		expect(results.find((r) => r.outcome === 'stale')).toMatchObject({ latest: 2 })
		expect((await dbMod.listArtifactVersions(created.id)).map((v) => v.version)).toEqual([2, 1])
	})

	it("keeps a chat's edit refused until it reads the version another tab stored", async () => {
		const other = new stateMod.SessionArtifactsStore()
		await store.setSession('s1')
		// Written by another chat, so the one editing it has read no version of it.
		const created = saved(await store.create('s1', { name: 'Doc', content: 'v1' }))
		await other.setSession('s1')
		saved(await other.edit(created.id, { content: 'v2 from the other tab' }))

		const edit = () => store.edit(created.id, { content: 'from the chat' }, { chatId: 'c1' })
		expect(await edit()).toMatchObject({ outcome: 'stale', latest: 2 })
		// Shown in this tab now, the latest version is still not one the chat has read.
		expect(store.artifacts[0]).toMatchObject({ version: 2 })
		expect(await edit()).toMatchObject({ outcome: 'stale', latest: 2 })

		store.markRead('c1', store.artifacts[0])
		expect(await edit()).toMatchObject({ outcome: 'saved', version: 3 })
	})

	it('says an edit the browser could not store was not stored, and keeps the version it holds', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Doc', content: 'v1' }))

		refuseNextWrite()
		expect(await store.edit(created.id, { content: 'v2' })).toEqual({
			outcome: 'unstored'
		})
		expect(store.artifacts[0]).toMatchObject({ content: 'v1', version: 1 })
		// The next change is made on the version stored, not on the one refused.
		expect(saved(await store.edit(created.id, { name: 'Renamed' }))).toMatchObject({
			name: 'Renamed',
			content: 'v1',
			version: 2
		})
	})

	it('refuses to create an artifact when there is no store to write to', async () => {
		// No scoped user, so the database never opens.
		;(await import('$lib/stores')).userStore.set(undefined as never)
		await store.setSession('s1')

		expect(await store.create('s1', { name: 'A', content: 'x' })).toEqual({
			outcome: 'unstored'
		})
		expect(store.artifacts).toEqual([])
	})

	it('get serves an artifact the list holds that this browser never stored', async () => {
		// A read-only view of a shared session fills the list itself.
		store.artifacts = [mk({ id: 'shown' })]
		expect((await store.get('shown'))?.id).toBe('shown')
	})

	it('get falls back to the DB for an artifact outside the loaded list', async () => {
		const created = saved(await store.create('s2', { name: 'B', content: 'y' }))
		await store.setSession('s1') // s2 not loaded
		expect((await store.get(created.id))?.id).toBe(created.id)
		expect(await store.get('nope')).toBeUndefined()
	})

	it('listForSession reads the DB for a session that is not loaded', async () => {
		await store.create('s2', { name: 'B', content: 'y' })
		await store.setSession('s1') // s2 not loaded
		expect((await store.listForSession('s2')).map((a) => a.name)).toEqual(['B'])
		expect(await store.listForSession('s1')).toEqual([])
	})

	it('refuses an edit of another session when scoped to one', async () => {
		const created = saved(await store.create('s2', { name: 'Off', content: 'v1' }))
		expect(await store.edit(created.id, { content: 'v2' }, { sessionId: 's1' })).toEqual({
			outcome: 'missing'
		})
		expect((await dbMod.getArtifact(created.id))?.content).toBe('v1')
		expect(await store.edit(created.id, { content: 'v2' }, { sessionId: 's2' })).toMatchObject({
			outcome: 'saved'
		})
	})

	it('tells an omitted field from an explicit empty string', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: 'body' }))

		// Omitting content leaves it untouched...
		expect(saved(await store.edit(created.id, { name: 'Renamed' })).content).toBe('body')
		// ...but an explicit empty string blanks it.
		expect(saved(await store.edit(created.id, { content: '' })).content).toBe('')
	})

	it('stores a version per change of the text or the name, and none for a write that changes neither', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: 'v1' }))

		await store.edit(created.id, { content: 'v2', note: 'Added a rollback step' })
		// A rename is a version: the picker shows the name each version had.
		await store.edit(created.id, { name: 'Renamed' })
		expect(
			await store.edit(created.id, { name: ' Renamed ', content: 'v2', note: 'ignored' })
		).toMatchObject({ outcome: 'unchanged' })

		const versions = await store.listVersions(created.id)
		expect(versions.map((v) => [v.version, v.name, v.content, v.note])).toEqual([
			[3, 'Renamed', 'v2', 'Renamed'],
			[2, 'Plan', 'v2', 'Added a rollback step'],
			[1, 'Plan', 'v1', undefined]
		])
		expect((await store.getVersion(created.id, 1))?.content).toBe('v1')
	})

	it('persists a row and the snapshots it produced in one write', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Doc', content: 'c1' }))

		const spy = vi.spyOn(dbMod, 'mutateArtifact')
		await store.edit(created.id, { content: 'c2', note: 'second' })

		// Split into two writes, a stamped version can outlive the snapshot that failed to
		// land beside it. That state still reads as complete, because listVersions
		// synthesizes the current version from the row — right up until the next edit
		// overwrites the row, which is the only copy of that content left.
		expect(spy).toHaveBeenCalledTimes(1)
		expect((await store.listVersions(created.id)).map((v) => v.version)).toEqual([2, 1])

		const row = await dbMod.getArtifact(created.id)
		const stored = await dbMod.listArtifactVersions(created.id)
		expect(stored.some((v) => v.version === row!.version)).toBe(true)
	})

	it('serves the live version from memory when the store is unreadable, but not an older one', async () => {
		await store.setSession('s1')
		const created = saved(await store.create('s1', { name: 'Plan', content: 'v1' }))
		await store.edit(created.id, { content: 'v2', note: 'second' })
		vi.spyOn(dbMod, 'getArtifactVersion').mockRejectedValue(new Error('read failed'))

		expect((await store.getVersion(created.id, 2))?.content).toBe('v2')
		await expect(store.getVersion(created.id, 1)).rejects.toThrow('read failed')
	})

	it('keeps a legacy v1 when a rename lands before the first edit of the text', async () => {
		await dbMod.putArtifact(mk({ id: 'legacy', content: 'original' }))
		await store.setSession('s1')

		// The rename stamps `version`, after which nothing else would recognise this as a
		// pre-history artifact — so its v1 has to be captured here or it is lost for good.
		await store.edit('legacy', { name: 'Renamed' })
		await store.edit('legacy', { content: 'edited', note: 'Rewrote it' })

		expect((await store.listVersions('legacy')).map((v) => [v.version, v.name, v.content])).toEqual(
			[
				[3, 'Renamed', 'edited'],
				[2, 'Renamed', 'original'],
				[1, 'Doc', 'original']
			]
		)
	})

	it('reads an artifact stored before history existed as its own version 1', async () => {
		await dbMod.putArtifact(mk({ id: 'legacy', content: 'only' }))
		await store.setSession('s1')

		expect(await store.listVersions('legacy')).toMatchObject([{ version: 1, content: 'only' }])
		expect((await store.getVersion('legacy', 1))?.content).toBe('only')
		// Its first edit still lands as v2, so version numbers stay monotonic.
		await store.edit('legacy', { content: 'edited' })
		expect((await store.listVersions('legacy')).map((v) => v.version)).toEqual([2, 1])
	})

	it('refuses a first proposal over a plan another tab created, until the chat reads it', async () => {
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
		const propose = () =>
			store.proposePlan('s1', { name: 'Mine', content: 'mine', note: 'proposed' }, 'c1')

		// What exit_plan_mode checks before its card agrees with what the save will answer.
		expect(await store.stalePlanVersion('s1', 'c1')).toBe(1)
		expect(await propose()).toMatchObject({ outcome: 'stale', latest: 1 })
		// Absent from here, it would stay out of the preview, the transcript's plan card and
		// list_artifacts until a reload.
		expect(store.artifacts.map((a) => [a.id, a.content])).toEqual([
			[dbMod.planArtifactId('s1'), 'v1']
		])
		expect(await propose()).toMatchObject({ outcome: 'stale', latest: 1 })

		store.markRead('c1', store.artifacts[0])
		expect(await store.stalePlanVersion('s1', 'c1')).toBeUndefined()
		expect(await propose()).toMatchObject({ outcome: 'saved', version: 2 })
	})

	it('says a plan, or an approval, the browser could not store was not stored', async () => {
		// Approving a plan that is gone on reload would leave the user agreeing to a document
		// nothing can show them.
		await store.setSession('s1')
		const first = { name: 'Plan', content: '# p', note: 'first' }
		refuseNextWrite()
		expect(await store.proposePlan('s1', first, undefined)).toEqual({
			outcome: 'unstored'
		})

		const plan = saved(await store.proposePlan('s1', first, undefined))
		refuseNextWrite()
		expect(await store.approve(plan.id, 1)).toBe(false)
		// An approval reflected in memory anyway would show the `plan` pill, and tell the model
		// the user signed off, until the next reload.
		expect(store.artifacts.find((a) => a.id === plan.id)?.approvedVersion).toBeUndefined()
		expect((await store.get(plan.id))?.approvedVersion).toBeUndefined()

		expect(await store.approve(plan.id, 1)).toBe(true)
		expect(store.artifacts.find((a) => a.id === plan.id)?.approvedVersion).toBe(1)
	})

	it('holds one plan per session, and frees the slot of a deleted plan', async () => {
		await store.setSession('s1')
		const plan = saved(await store.create('s1', { name: 'Plan', content: 'x', role: 'plan' }))
		await store.approve(plan.id, 1)
		expect(await store.create('s1', { name: 'Other', content: 'y', role: 'plan' })).toMatchObject({
			outcome: 'plan-taken',
			plan: { id: plan.id }
		})
		// Another session's slot is its own.
		expect(
			await store.create('s2', { name: 'Elsewhere', content: 'z', role: 'plan' })
		).toMatchObject({ outcome: 'saved' })

		await store.remove(plan.id)
		const next = saved(await store.create('s1', { name: 'Next', content: 'w', role: 'plan' }))
		expect(next).toMatchObject({ id: plan.id, name: 'Next', version: 1, role: 'plan' })
		expect(next.approvedVersion).toBeUndefined()
		expect(store.artifacts.map((a) => a.id)).toContain(plan.id)
	})

	it('stores only the first of two proposals two tabs make at once on one plan', async () => {
		const other = new stateMod.SessionArtifactsStore()
		await store.setSession('s1')
		saved(await store.proposePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined))
		await other.setSession('s1')

		const results = await Promise.all([
			store.proposePlan('s1', { name: 'Plan', content: 'from A', note: 'A' }, undefined),
			other.proposePlan('s1', { name: 'Plan', content: 'from B', note: 'B' }, undefined)
		])

		expect(results.map((r) => r.outcome).sort()).toEqual(['saved', 'stale'])
		const versions = (await store.listVersions(dbMod.planArtifactId('s1'))).map((v) => v.version)
		expect(versions).toEqual([2, 1])
	})

	it('stores only the first of two first proposals two tabs make at once', async () => {
		// The second never read the plan the first created.
		const other = new stateMod.SessionArtifactsStore()
		await store.setSession('s1')
		await other.setSession('s1')

		const results = await Promise.all([
			store.proposePlan('s1', { name: 'Plan', content: 'from A', note: 'A' }, undefined),
			other.proposePlan('s1', { name: 'Plan', content: 'from B', note: 'B' }, undefined)
		])

		expect(results.map((r) => r.outcome).sort()).toEqual(['saved', 'stale'])
		expect(results.find((r) => r.outcome === 'stale')).toMatchObject({ latest: 1 })
		const versions = (await store.listVersions(dbMod.planArtifactId('s1'))).map((v) => v.version)
		expect(versions).toEqual([1])
	})

	it('an approval racing a newer proposal moves the pointer without reverting content', async () => {
		// approve() must patch, never rewrite: an approval computed while another tab was
		// revising would otherwise carry this tab's older text back over the newer one.
		const other = new stateMod.SessionArtifactsStore()
		await store.setSession('s1')
		const plan = saved(
			await store.proposePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined)
		)
		await other.setSession('s1')

		saved(
			await other.proposePlan(
				's1',
				{ name: 'Plan', content: 'v2 from the other tab', note: 'B' },
				undefined
			)
		)
		await store.approve(plan.id, 1)

		const row = await dbMod.getArtifact(plan.id)
		expect(row).toMatchObject({ content: 'v2 from the other tab', version: 2, approvedVersion: 1 })
	})

	it('leaves the plan alone for a caller whose policy refuses it, and only that caller', async () => {
		await store.setSession('s1')
		const plan = saved(
			await store.proposePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined)
		)
		const doc = saved(await store.create('s1', { name: 'Doc', content: 'x' }))
		const no = { canWritePlan: () => false }

		expect(await store.edit(plan.id, { content: 'rewritten' }, no)).toEqual({
			outcome: 'plan-locked'
		})
		expect(await store.create('s2', { name: 'Plan', content: 'x', role: 'plan' }, no)).toEqual({
			outcome: 'plan-locked'
		})
		// The policy is about the plan, not about writing: everything else is untouched by it.
		expect(saved(await store.edit(doc.id, { content: 'y' }, no)).content).toBe('y')
		// And a caller that states no policy keeps every posture's plan writes exactly as they were.
		expect(await store.edit(plan.id, { content: 'v2' })).toMatchObject({
			outcome: 'saved',
			version: 2
		})
		expect((await dbMod.getArtifact(plan.id))?.content).toBe('v2')
	})

	it('refuses an approval naming no readable version, or no plan at all', async () => {
		await store.setSession('s1')
		const plan = saved(
			await store.proposePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined)
		)
		const doc = saved(await store.create('s1', { name: 'Doc', content: 'x' }))

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
		const plan = saved(
			await store.proposePlan('s1', { name: 'Plan', content: 'v1', note: 'first' }, undefined)
		)
		await store.proposePlan('s1', { name: 'Plan', content: 'v2', note: 'second' }, undefined)
		await store.proposePlan('s1', { name: 'Plan', content: 'v3', note: 'third' }, undefined)

		await expect(store.approve(plan.id, 1)).resolves.toBe(true)
		expect(await dbMod.getArtifact(plan.id)).toMatchObject({ version: 3, approvedVersion: 1 })
	})

	it('carries an approval only onto a new text, and only when asked', async () => {
		// A plan approved at v1 and revised into a proposal the user turned down: a rename, or a
		// rewrite with the text already there, would otherwise promote the refused text.
		await store.setSession('s1')
		const plan = saved(await store.create('s1', { name: 'Plan', content: 'v1', role: 'plan' }))
		await store.approve(plan.id, 1)
		expect(saved(await store.edit(plan.id, { content: 'v2 the user rejected' }))).toMatchObject({
			version: 2,
			approvedVersion: 1
		})

		expect(
			saved(await store.edit(plan.id, { name: 'Plan, renamed', keepApproved: true }))
		).toMatchObject({ version: 3, approvedVersion: 1 })
		expect(
			await store.edit(plan.id, { content: 'v2 the user rejected', keepApproved: true })
		).toMatchObject({ outcome: 'unchanged' })

		// A new text still carries it: an edit outside plan mode is one the user's posture
		// already trusts.
		expect(saved(await store.edit(plan.id, { content: 'v4', keepApproved: true }))).toMatchObject({
			version: 4,
			approvedVersion: 4
		})
		// A rename of the version the user approved is still that plan.
		expect(
			saved(await store.edit(plan.id, { name: 'Plan, renamed again', keepApproved: true }))
		).toMatchObject({ version: 5, approvedVersion: 5 })
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
