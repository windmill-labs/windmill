import { describe, it, expect, beforeEach, vi } from 'vitest'

// The userStore subscription is gated on BROWSER; the vitest "server" env reports BROWSER=false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))
const session = vi.hoisted(() => ({ identity: 'a@x.com' as string | undefined, hint: 'h1' }))
vi.mock('$lib/persistedCache.svelte', () => ({ cacheIdentity: () => session.identity }))
vi.mock('$lib/lastSession', () => ({
	readSessionHint: () => session.hint,
	sameSessionSince: (sentWith: string | undefined) => session.hint === sentWith
}))
vi.mock('$lib/gen', () => ({ ScriptService: { listRunnables: vi.fn() } }))

import { ScriptService } from '$lib/gen'
import { userStore, type UserExt } from '$lib/stores'
import { recentItems, type RecentItem } from './recentItems.svelte'

const WS = 'ws'
const KEY = `homeRecentItems:${WS}::a@x.com`
type Live = { script: string[]; flow: string[]; app: string[]; agent: string[] }
const none: Live = { script: [], flow: [], app: [], agent: [] }

function stored(): RecentItem[] {
	return JSON.parse(localStorage.getItem(KEY) ?? '[]')
}
function paths(): string[] {
	return recentItems.list(WS).map((x) => `${x.kind}:${x.path}`)
}
/** The check's reply, held until `answer` is called, so a test can act while it is out. */
function heldReply() {
	let answer!: (live: Live) => void
	let fail!: (e: unknown) => void
	vi.mocked(ScriptService.listRunnables).mockImplementationOnce(
		() =>
			new Promise((resolve, reject) => {
				answer = resolve
				fail = reject
			}) as any
	)
	const items = (live: Live) =>
		Object.entries(live).flatMap(([type, paths]) => paths.map((path) => ({ type, path })))
	return {
		answer: (live: Live) => answer({ items: items(live) } as any),
		fail: (e: unknown) => fail(e)
	}
}

beforeEach(() => {
	// Every reading is later than the last, so "before the check was sent" is never the same
	// millisecond as it (which counts as after, and would keep everything).
	let now = Date.UTC(2026, 0, 1)
	vi.spyOn(Date, 'now').mockImplementation(() => ++now)
	localStorage.clear()
	session.identity = 'a@x.com'
	session.hint = 'h1'
	userStore.set({ email: 'a@x.com', username: 'a' } as unknown as UserExt)
	vi.mocked(ScriptService.listRunnables).mockReset()
})

describe('rename', () => {
	it('moves the item in place, keeping its spot and time, with its new summary', () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', summary: 'Old', via: 'view' })
		recentItems.record(WS, { kind: 'flow', path: 'u/a/two', via: 'view' })
		const before = stored()
		recentItems.rename(WS, 'script', 'u/a/one', 'u/a/renamed', 'New')
		expect(paths()).toEqual(['flow:u/a/two', 'script:u/a/renamed'])
		expect(stored()[1]).toMatchObject({ at: before[1].at, summary: 'New' })
	})

	it('updates the summary of a summary-only save without marking it moved', () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', summary: 'Old', via: 'view' })
		recentItems.rename(WS, 'script', 'u/a/one', 'u/a/one', 'New')
		expect(paths()).toEqual(['script:u/a/one'])
		expect(stored()[0].summary).toBe('New')
		expect(stored()[0].movedAt).toBeUndefined()
	})

	it('keeps the renamed item over a gone one already at the new path', () => {
		recentItems.record(WS, { kind: 'raw_app', path: 'u/a/to', summary: 'Gone', via: 'view' })
		recentItems.record(WS, { kind: 'app', path: 'u/a/from', summary: 'Kept', via: 'view' })
		recentItems.rename(WS, 'app', 'u/a/from', 'u/a/to', 'Kept')
		expect(stored()).toMatchObject([{ kind: 'app', path: 'u/a/to', summary: 'Kept' }])
	})

	it('treats an app and a raw app at one path as one item', () => {
		recentItems.record(WS, { kind: 'raw_app', path: 'u/a/app', via: 'view' })
		recentItems.record(WS, { kind: 'app', path: 'u/a/app', via: 'view' })
		expect(paths()).toEqual(['app:u/a/app'])
		// The shared rename function only knows `app`.
		recentItems.record(WS, { kind: 'raw_app', path: 'u/a/raw', via: 'view' })
		recentItems.rename(WS, 'app', 'u/a/raw', 'u/a/raw2', '')
		expect(paths()).toContain('raw_app:u/a/raw2')
	})

	it('waits for the user, like record', () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', via: 'view' })
		userStore.set(undefined)
		recentItems.rename(WS, 'script', 'u/a/one', 'u/a/two', '')
		userStore.set({ email: 'a@x.com', username: 'a' } as unknown as UserExt)
		expect(paths()).toEqual(['script:u/a/two'])
	})
})

describe('prune', () => {
	it('drops what the server no longer has, per kind, and keeps the rest', async () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/shared', via: 'view' })
		recentItems.record(WS, { kind: 'flow', path: 'u/a/shared', via: 'view' })
		recentItems.record(WS, { kind: 'raw_app', path: 'u/a/app', via: 'view' })
		const reply = heldReply()
		const done = recentItems.prune(WS)
		reply.answer({ ...none, flow: ['u/a/shared'], app: ['u/a/app'] })
		await done
		expect(paths()).toEqual(['raw_app:u/a/app', 'flow:u/a/shared'])
		expect(vi.mocked(ScriptService.listRunnables).mock.calls[0][0]).toMatchObject({
			paths: 'u/a/app,u/a/shared'
		})
	})

	it('keeps what was opened or moved, here or in another tab, while the check was out', async () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/opened', via: 'view' })
		recentItems.record(WS, { kind: 'script', path: 'u/a/live', via: 'view' })
		recentItems.record(WS, { kind: 'script', path: 'u/a/stale', via: 'view' })
		const reply = heldReply()
		const done = recentItems.prune(WS)
		recentItems.record(WS, { kind: 'script', path: 'u/a/opened', via: 'view' })
		// Another tab moves `live` onto `stale`'s path: its write is all this tab sees.
		const movedAt = Date.now()
		localStorage.setItem(
			KEY,
			JSON.stringify(
				stored()
					.filter((x) => x.path !== 'u/a/live')
					.map((x) => (x.path === 'u/a/stale' ? { ...x, movedAt } : x))
			)
		)
		// The answer predates both: `opened`, `live` and `stale` were all gone when asked.
		reply.answer({ ...none })
		await done
		expect(paths()).toEqual(['script:u/a/opened', 'script:u/a/stale'])
	})

	it.each([
		['the check fails', (r: ReturnType<typeof heldReply>) => r.fail(new Error('down'))],
		[
			'the user changed meanwhile',
			(r: ReturnType<typeof heldReply>) => {
				session.identity = 'b@y.com'
				r.answer({ ...none })
			}
		],
		[
			'another tab swapped the session meanwhile',
			(r: ReturnType<typeof heldReply>) => {
				session.hint = 'h2'
				r.answer({ ...none })
			}
		]
	])('drops nothing when %s', async (_, settle) => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', via: 'view' })
		const reply = heldReply()
		const done = recentItems.prune(WS)
		settle(reply)
		await done
		expect(paths()).toEqual(['script:u/a/one'])
	})

	it('ignores an answer a later check superseded', async () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', via: 'view' })
		const older = heldReply()
		const first = recentItems.prune(WS)
		const newer = heldReply()
		const second = recentItems.prune(WS)
		older.answer({ ...none })
		await first
		expect(paths()).toEqual(['script:u/a/one'])
		newer.answer({ ...none, script: ['u/a/one'] })
		await second
		expect(paths()).toEqual(['script:u/a/one'])
	})

	it('sends nothing without a confirmed session', async () => {
		recentItems.record(WS, { kind: 'script', path: 'u/a/one', via: 'view' })
		session.identity = undefined
		await recentItems.prune(WS)
		expect(ScriptService.listRunnables).not.toHaveBeenCalled()
		expect(paths()).toEqual(['script:u/a/one'])
	})
})
