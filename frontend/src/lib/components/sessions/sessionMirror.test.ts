import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'

// The stores are BROWSER-gated; the vitest "server" env reports false.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

const { pushMock, listMock, pullMock } = vi.hoisted(() => ({
	pushMock: vi.fn(),
	listMock: vi.fn(),
	pullMock: vi.fn()
}))
vi.mock('$lib/gen', async (orig) => {
	const actual = await orig<typeof import('$lib/gen')>()
	return {
		...actual,
		AiService: {
			...actual.AiService,
			pushAiSessionBackups: pushMock,
			listAiSessionBackups: listMock,
			pullAiSessionBackups: pullMock
		},
		WorkspaceService: {
			...actual.WorkspaceService,
			listUserWorkspaces: vi.fn().mockResolvedValue([]),
			getSessionWorkspaceStatus: vi.fn().mockResolvedValue({})
		}
	}
})

// The chat store during a restore: real, or unreachable for one session.
const { chatImport } = vi.hoisted(() => ({ chatImport: { unavailable: false } }))
vi.mock('../copilot/chat/HistoryManager.svelte', async (orig) => {
	const actual = await orig<typeof import('../copilot/chat/HistoryManager.svelte')>()
	return {
		...actual,
		importStoredChats: (...args: Parameters<typeof actual.importStoredChats>) =>
			chatImport.unavailable ? Promise.resolve(false) : actual.importStoredChats(...args)
	}
})

import { superadmin, userStore, usersWorkspaceStore, type UserExt } from '$lib/stores'
import HistoryManager, {
	__resetBackupStoreForTesting,
	__resetLegacyChatClaimForTesting,
	readStoredChat
} from '../copilot/chat/HistoryManager.svelte'
import {
	deleteSession,
	importSessions,
	putSession,
	sessionState,
	type Session
} from './sessionState.svelte'
import { markSessionDirty } from './sessionMirrorSignal'
import {
	__flushForTesting,
	__resetMirrorForTesting,
	__settleForTesting,
	__syncRowsForTesting,
	__writeSyncForTesting,
	restoreSessionBackups
} from './sessionMirror.svelte'

const EMAIL = 'mirror@x.com'
const IMAGE = 'data:image/png;base64,AAAA'
const PENDING_PREFIX = `windmill_sessions_mirror_pending::${EMAIL}::`

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}
const flush = () => new Promise<void>((r) => setTimeout(r, 0))

function pendingKeys(): string[] {
	const keys: string[] = []
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i)
		if (key?.startsWith(PENDING_PREFIX)) keys.push(key.slice(PENDING_PREFIX.length))
	}
	return keys.sort()
}

function removalKeys(): string[] {
	return pendingKeys().filter((k) => k.startsWith('r::'))
}

/** Sessions whose dirty counter no push has covered yet. */
async function pendingDirty(): Promise<string[]> {
	const flushed = new Map<string, number>()
	for (const r of await __syncRowsForTesting(EMAIL)) {
		flushed.set(r.id, r.stale ? -1 : (r.flushedV ?? -1) - (r.extraV ?? 0))
	}
	const out: string[] = []
	for (const key of pendingKeys()) {
		if (!key.startsWith('d::')) continue
		const id = key.slice(3)
		if (Number(localStorage.getItem(PENDING_PREFIX + key)) > (flushed.get(id) ?? -1)) out.push(id)
	}
	return out.sort()
}

beforeEach(async () => {
	;(globalThis as any).indexedDB = new IDBFactory()
	localStorage.clear()
	__resetLegacyChatClaimForTesting()
	__resetBackupStoreForTesting()
	__resetMirrorForTesting()
	pushMock.mockReset()
	listMock.mockReset()
	pullMock.mockReset()
	chatImport.unavailable = false
	superadmin.set(false)
	usersWorkspaceStore.set(undefined)
	userStore.set(undefined)
	await flush()
	sessionState.sessions = []
	userStore.set(asUser(EMAIL))
	await vi.waitFor(() => expect(sessionState.hydrated).toBe(true))
})

// The whole loop, end to end against the real stores: local writes mark, a flush sends
// one batch carrying the record, the chat and its image, reading the session sends
// nothing, and a user delete removes the backup.
describe('sessionMirror flush', () => {
	it('pushes what changed, once, and removes a deleted session', async () => {
		pushMock.mockResolvedValue({ enabled: true, results: [{ id: 's1' }] })
		const s: Session = {
			id: 's1',
			name: 'session-1',
			createdAt: 1,
			workspace_id: 'ws',
			chatId: 'c1'
		}
		sessionState.sessions = [s]
		await putSession(s)

		const hm = new HistoryManager()
		await hm.init()
		hm.setSessionId('s1')
		hm.setCurrentChatId('c1')
		await hm.saveChat(
			[{ role: 'user', content: 'hello', images: [{ dataUrl: IMAGE, name: 'a.png' }] } as never],
			[{ role: 'user', content: [{ type: 'image_url', image_url: { url: IMAGE } }] } as never]
		)

		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		const body = pushMock.mock.calls[0][0].requestBody
		expect(pushMock.mock.calls[0][0].workspace).toBe('ws')
		expect(body.owner).toBe(EMAIL)
		expect(body.removed).toBeUndefined()
		const [imageEntry, entry] = body.sessions
		expect(imageEntry.images).toEqual([{ chat_id: 'c1', id: expect.any(String), data_url: IMAGE }])
		// A first push goes whole: the head opens it on the first part, whichever that is.
		expect(imageEntry.whole).toBe(true)
		expect(imageEntry.partial).toBe(true)
		expect(imageEntry.head).toEqual({ id: 's1', createdAt: 1, workspace_id: 'ws', chatId: 'c1' })
		expect(entry.head).toBeUndefined()
		expect(entry.whole).toBeUndefined()
		expect(entry.chats.map((c: { id: string }) => c.id)).toEqual(['c1'])
		// The record keeps its blob ref; bytes travel as the image object only.
		expect(JSON.stringify(entry.chats[0].record)).not.toContain(IMAGE)
		expect(entry.artifacts).toBeUndefined()
		expect(await pendingDirty()).toEqual([])

		// Reading the session is not a change the backup keeps.
		await putSession({ ...s, lastSeenCount: 2, lastActivityAt: 99 })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// A user delete takes the backup with it.
		deleteSession('s1')
		await flush()
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody).toEqual({
			owner: EMAIL,
			sessions: [],
			removed: ['s1']
		})
		expect(pendingKeys()).toEqual([])
		hm.close()
	})

	it('carries a user delete on the sync row when its localStorage mark cannot be written', async () => {
		pushMock.mockResolvedValue({ enabled: true, results: [{ id: 'sd' }] })
		const s: Session = { id: 'sd', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// Storage full at the moment of the delete.
		const setItem = localStorage.setItem.bind(localStorage)
		localStorage.setItem = (key: string, value: string) => {
			if (key.includes('::r::')) throw new Error('QuotaExceededError')
			setItem(key, value)
		}
		try {
			deleteSession('sd')
		} finally {
			localStorage.setItem = setItem
		}
		await flush()
		expect(removalKeys()).toEqual([])
		await vi.waitFor(async () =>
			expect((await __syncRowsForTesting(EMAIL)).find((r) => r.id === 'sd')?.removed).toBe(true)
		)

		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody).toEqual({
			owner: EMAIL,
			sessions: [],
			removed: ['sd']
		})
		expect((await __syncRowsForTesting(EMAIL)).some((r) => r.id === 'sd')).toBe(false)
		await __settleForTesting()
	})

	it('keeps a delete filed on the sync row while the first push is still in flight', async () => {
		const s: Session = { id: 'sr', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)
		let release!: (value: unknown) => void
		pushMock.mockImplementationOnce(() => new Promise((r) => (release = r)))
		const inFlight = __flushForTesting()
		await vi.waitFor(() => expect(pushMock).toHaveBeenCalledTimes(1))

		// Storage full at the moment of the delete, the push not yet answered.
		const setItem = localStorage.setItem.bind(localStorage)
		localStorage.setItem = (key: string, value: string) => {
			if (key.includes('::r::')) throw new Error('QuotaExceededError')
			setItem(key, value)
		}
		try {
			deleteSession('sr')
		} finally {
			localStorage.setItem = setItem
		}
		await vi.waitFor(async () =>
			expect((await __syncRowsForTesting(EMAIL)).find((r) => r.id === 'sr')?.removed).toBe(true)
		)
		release({ enabled: true, results: [{ id: 'sr' }] })
		await inFlight
		// The push's own row write did not lose the removal.
		expect((await __syncRowsForTesting(EMAIL)).find((r) => r.id === 'sr')?.removed).toBe(true)

		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sr' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody.removed).toEqual(['sr'])
		expect((await __syncRowsForTesting(EMAIL)).some((r) => r.id === 'sr')).toBe(false)
	})

	it('keeps the marks when the push fails, and stops for a workspace without storage', async () => {
		const s: Session = { id: 's2', name: 'session-2', createdAt: 1, workspace_id: 'ws' }
		const never: Session = { id: 's2b', name: 'session-3', createdAt: 2, workspace_id: 'ws' }
		sessionState.sessions = [s, never]
		await putSession(s)

		pushMock.mockRejectedValueOnce(new TypeError('network'))
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(await pendingDirty()).toEqual(['s2'])
		// Still marked: the retry carries it again once the backoff lapses.
		__resetMirrorForTesting()
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 's2' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(await pendingDirty()).toEqual([])

		// The storage goes away: no further request for the page. A delete keeps its
		// removal mark only for a session that was backed up (s2), for when backups are on
		// again, or it would come back from the bucket; one never backed up has nothing
		// there, so its mark goes rather than piling up on a storage-less instance.
		await putSession({ ...s, summary: 'changed' })
		await putSession(never)
		pushMock.mockResolvedValueOnce({ enabled: false, results: [] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		// The reconcile that runs after login may have re-read the list from the store
		// before `never` was in it; deleteSession only acts on sessions it can see.
		sessionState.sessions = [s, never]
		deleteSession('s2')
		deleteSession('s2b')
		await flush()
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		expect(pendingKeys()).toEqual(['r::s2::ws'])
	})

	it('settles nothing of a request that failed, even a session whose parts were still to come', async () => {
		// Enough sessions for two requests: the first fails, the second is never sent.
		const ids = Array.from({ length: 101 }, (_, i) => `m${i}`)
		for (const id of ids) {
			await putSession({ id, name: id, createdAt: 1, workspace_id: 'ws' })
		}
		pushMock.mockRejectedValueOnce(new TypeError('network'))
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(pushMock.mock.calls[0][0].requestBody.sessions).toHaveLength(100)
		expect(await pendingDirty()).toHaveLength(101)

		// Once the backoff lapses, every one of them is carried again.
		__resetMirrorForTesting()
		pushMock.mockImplementation(
			async ({ requestBody }: { requestBody: { sessions: { id: string }[] } }) => ({
				enabled: true,
				results: requestBody.sessions.map((s) => ({ id: s.id }))
			})
		)
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		expect(
			pushMock.mock.calls
				.slice(1)
				.flatMap((c) => c[0].requestBody.sessions.map((s: { id: string }) => s.id))
				.sort()
		).toEqual([...ids].sort())
		expect(await pendingDirty()).toEqual([])
	})

	it('backs off when the server could not store a session, keeping its mark', async () => {
		const s: Session = { id: 's3', name: 'session-3', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)

		pushMock.mockResolvedValue({ enabled: true, results: [{ id: 's3', error: 'bucket refused' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		// Still marked, but not re-sent until the backoff lapses.
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(await pendingDirty()).toEqual(['s3'])
	})

	it('fails only the sessions of a request the server found too large', async () => {
		const ids = Array.from({ length: 101 }, (_, i) => `t${i}`)
		for (const id of ids) {
			await putSession({ id, name: id, createdAt: 1, workspace_id: 'ws' })
		}
		const { ApiError } = await import('$lib/gen')
		pushMock
			.mockRejectedValueOnce(
				new ApiError({ method: 'POST', url: '' } as never, { status: 413 } as never, 'too large')
			)
			.mockImplementation(
				async ({ requestBody }: { requestBody: { sessions: { id: string }[] } }) => ({
					enabled: true,
					results: requestBody.sessions.map((s) => ({ id: s.id }))
				})
			)
		await __flushForTesting()
		// The second request still went out and settled its session; the first's stay marked.
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(await pendingDirty()).toHaveLength(100)
		const settled = pushMock.mock.calls[1][0].requestBody.sessions[0].id
		expect(await pendingDirty()).not.toContain(settled)
	})

	it('moves a session whole into its new workspace and files the old copy for removal', async () => {
		const s: Session = { id: 'mv', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'mv' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// The old workspace's backups go off before the move: its rows go stale, but they
		// still say where the copy is.
		await putSession({ ...s, summary: 'changed' })
		pushMock.mockResolvedValueOnce({ enabled: false, results: [] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)

		await putSession({ ...s, summary: 'changed', workspace_id: 'ws2' })
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'mv' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		expect(pushMock.mock.calls[2][0].workspace).toBe('ws2')
		expect(pushMock.mock.calls[2][0].requestBody.sessions[0].head.workspace_id).toBe('ws2')
		// The removal waits for the old workspace's backups to be on again.
		expect(removalKeys()).toEqual(['r::mv::ws'])
		expect(await pendingDirty()).toEqual([])
	})

	it('pushes every session whole again once the server answers from another storage', async () => {
		const a: Session = {
			id: 'sa',
			name: 'session-1',
			createdAt: 1,
			workspace_id: 'ws',
			chatId: 'ca'
		}
		const b: Session = { id: 'sb', name: 'session-2', createdAt: 2, workspace_id: 'ws' }
		sessionState.sessions = [a, b]
		await putSession(a)
		await putSession(b)
		const hm = new HistoryManager()
		await hm.init()
		hm.setSessionId('sa')
		hm.setCurrentChatId('ca')
		await hm.saveChat(
			[{ role: 'user', content: 'hello' } as never],
			[{ role: 'user', content: 'hello' } as never]
		)
		pushMock.mockResolvedValueOnce({
			enabled: true,
			storage_id: 'bucket-1',
			results: [{ id: 'sa' }, { id: 'sb' }]
		})
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// Only `sa`'s record changes, and the answer names a new storage: it holds `sb`
		// nowhere and `sa` only in the part that went, so both are backed up whole again.
		await putSession({ ...a, summary: 'changed' })
		pushMock.mockResolvedValueOnce({
			enabled: true,
			storage_id: 'bucket-2',
			results: [{ id: 'sa' }]
		})
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody.sessions[0].chats).toBeUndefined()
		expect(await pendingDirty()).toEqual(['sa', 'sb'])

		pushMock.mockResolvedValueOnce({
			enabled: true,
			storage_id: 'bucket-2',
			results: [{ id: 'sa' }, { id: 'sb' }]
		})
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		const entries = pushMock.mock.calls[2][0].requestBody.sessions
		expect(entries.map((s: { id: string }) => s.id).sort()).toEqual(['sa', 'sb'])
		expect(entries.every((s: { head?: unknown }) => s.head !== undefined)).toBe(true)
		expect(entries.find((s: { id: string }) => s.id === 'sa').chats).toHaveLength(1)
		expect(await pendingDirty()).toEqual([])
	})

	/** A session with three chats of 1.5 MB (record ~3 MB each): its entry splits past the
	 * 8 MB request target, so it spans two requests. */
	async function splitSession(id: string): Promise<void> {
		const s: Session = { id, name: 'session-1', createdAt: 1, workspace_id: 'ws', chatId: 'c1' }
		sessionState.sessions = [s]
		await putSession(s)
		const hm = new HistoryManager()
		await hm.init()
		hm.setSessionId(id)
		const big = 'x'.repeat(1.5 * 1024 * 1024)
		for (const cid of ['c1', 'c2', 'c3']) {
			hm.setCurrentChatId(cid)
			await hm.saveChat(
				[{ role: 'user', content: big } as never],
				[{ role: 'user', content: big } as never]
			)
		}
	}

	it('settles nothing of a session whose parts were answered from different storages', async () => {
		await splitSession('sp')
		pushMock
			.mockResolvedValueOnce({ enabled: true, storage_id: 'bucket-1', results: [{ id: 'sp' }] })
			.mockResolvedValueOnce({ enabled: true, storage_id: 'bucket-2', results: [{ id: 'sp' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		// The first part opens the session whole with its head; only the last completes
		// the entry server-side.
		const [first, last] = pushMock.mock.calls.map((c) => c[0].requestBody.sessions[0])
		expect(first.partial).toBe(true)
		expect(first.whole).toBe(true)
		expect(first.head).toBeDefined()
		expect(last.partial).toBeUndefined()
		expect(last.whole).toBeUndefined()
		expect(last.head).toBeUndefined()
		expect(await pendingDirty()).toEqual(['sp'])
		expect(await __syncRowsForTesting(EMAIL)).toEqual([])
	})

	it('retires a removal only once the storage holding the backup answered it', async () => {
		const s: Session = { id: 'sr2', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)
		pushMock.mockResolvedValueOnce({ enabled: true, storage_id: 'A', results: [{ id: 'sr2' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// The workspace moved to another storage before the delete: the copy in A stays.
		deleteSession('sr2')
		await flush()
		pushMock.mockResolvedValueOnce({ enabled: true, storage_id: 'B', results: [{ id: 'sr2' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(removalKeys()).toEqual(['r::sr2::ws'])
		expect((await __syncRowsForTesting(EMAIL)).some((r) => r.id === 'sr2')).toBe(true)

		// Back on A, the removal lands.
		pushMock.mockResolvedValueOnce({ enabled: true, storage_id: 'A', results: [{ id: 'sr2' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		expect(pushMock.mock.calls[2][0].requestBody.removed).toEqual(['sr2'])
		expect(removalKeys()).toEqual([])
		expect((await __syncRowsForTesting(EMAIL)).some((r) => r.id === 'sr2')).toBe(false)
	})

	it('takes a bumped backup generation as a new storage for the rows, not for a removal', async () => {
		const a: Session = { id: 'ga', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		const b: Session = { id: 'gb', name: 'session-2', createdAt: 2, workspace_id: 'ws' }
		sessionState.sessions = [a, b]
		await putSession(a)
		await putSession(b)
		pushMock.mockResolvedValueOnce({
			enabled: true,
			storage_id: 'A',
			backup_generation: 0,
			results: [{ id: 'ga' }, { id: 'gb' }]
		})
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// The key was rotated before the delete: the removal is done (the old generation is
		// gone with the rotation), and `gb` goes whole again under the new one.
		deleteSession('ga')
		await flush()
		pushMock.mockResolvedValueOnce({
			enabled: true,
			storage_id: 'A',
			backup_generation: 1,
			results: [{ id: 'ga' }]
		})
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody.removed).toEqual(['ga'])
		expect(removalKeys()).toEqual([])
		expect(await pendingDirty()).toEqual(['gb'])
	})

	it('sends a session whole again once the server says its head is gone', async () => {
		const s: Session = {
			id: 'sh',
			name: 'session-1',
			createdAt: 1,
			workspace_id: 'ws',
			chatId: 'c1'
		}
		sessionState.sessions = [s]
		await putSession(s)
		const hm = new HistoryManager()
		await hm.init()
		hm.setSessionId('sh')
		hm.setCurrentChatId('c1')
		await hm.saveChat(
			[{ role: 'user', content: 'a' } as never],
			[{ role: 'user', content: 'a' } as never]
		)
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sh' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// Another device removed the backup; the next chat-only push finds no head there.
		hm.setCurrentChatId('c2')
		await hm.saveChat(
			[{ role: 'user', content: 'ab' } as never],
			[{ role: 'user', content: 'ab' } as never]
		)
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sh', needs_whole: true }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pushMock.mock.calls[1][0].requestBody.sessions[0].whole).toBeUndefined()
		expect(await pendingDirty()).toEqual(['sh'])
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sh' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		const whole = pushMock.mock.calls[2][0].requestBody.sessions[0]
		expect(whole.whole).toBe(true)
		expect(whole.head).toBeDefined()
		expect(whole.chats.map((c: { id: string }) => c.id).sort()).toEqual(['c1', 'c2'])
		expect(await pendingDirty()).toEqual([])
		hm.close()
	})

	it('holds the rest of a session back once the server refused a part of it', async () => {
		await splitSession('sf')
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sf', error: 'boom' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(await pendingDirty()).toEqual(['sf'])
		expect(await __syncRowsForTesting(EMAIL)).toEqual([])
	})

	it('carries an edit on the sync row when its dirty mark cannot be written, even during a push', async () => {
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sm' }] })
		const s: Session = { id: 'sm', name: 'session-1', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)

		// An edit's push is held open; storage is full for the edit that follows.
		let release!: (value: unknown) => void
		pushMock.mockImplementationOnce(() => new Promise((r) => (release = r)))
		await putSession({ ...s, summary: 'first' })
		const inFlight = __flushForTesting()
		await vi.waitFor(() => expect(pushMock).toHaveBeenCalledTimes(2))
		const setItem = localStorage.setItem.bind(localStorage)
		localStorage.setItem = (key: string, value: string) => {
			if (key.includes('::d::')) throw new Error('QuotaExceededError')
			setItem(key, value)
		}
		try {
			await putSession({ ...s, summary: 'second' })
		} finally {
			localStorage.setItem = setItem
		}
		await vi.waitFor(async () =>
			expect((await __syncRowsForTesting(EMAIL)).find((r) => r.id === 'sm')?.extraV).toBe(1)
		)
		release({ enabled: true, results: [{ id: 'sm' }] })
		await inFlight
		// The push's own row write kept the bump, so the later edit is still pending.
		expect(await pendingDirty()).toEqual(['sm'])
		pushMock.mockResolvedValueOnce({ enabled: true, results: [{ id: 'sm' }] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(3)
		expect(pushMock.mock.calls[2][0].requestBody.sessions[0].head.summary).toBe('second')
		expect(await pendingDirty()).toEqual([])
	})

	it('files a mark under the user whose store the write landed in', async () => {
		markSessionDirty('sw', undefined, 'other@x.com')
		expect(localStorage.getItem('windmill_sessions_mirror_pending::other@x.com::d::sw')).toBe('1')
		expect(pendingKeys()).toEqual([])

		// The mark of another user that cannot be written goes to that user's own rows.
		const { openDB } = await import('idb')
		const theirs = await openDB('windmill-sessions-mirror::other@x.com', 1, {
			upgrade: (db) => db.createObjectStore('sync', { keyPath: 'id' })
		})
		await theirs.put('sync', { id: 'so', ws: 'ws', head: 'h', chats: {}, images: {}, flushedV: 1 })
		theirs.close()
		const setItem = localStorage.setItem.bind(localStorage)
		localStorage.setItem = (key: string, value: string) => {
			if (key.includes('::d::')) throw new Error('QuotaExceededError')
			setItem(key, value)
		}
		try {
			markSessionDirty('so', undefined, 'other@x.com')
		} finally {
			localStorage.setItem = setItem
		}
		await vi.waitFor(async () => {
			const db = await openDB('windmill-sessions-mirror::other@x.com', 1)
			try {
				expect((await db.get('sync', 'so'))?.extraV).toBe(1)
			} finally {
				db.close()
			}
		})
	})

	it('keeps the marks when the server refuses a request, for the next page load', async () => {
		const s: Session = { id: 's4', name: 'session-4', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)

		const { ApiError } = await import('$lib/gen')
		pushMock.mockRejectedValue(
			new ApiError({ method: 'POST', url: '' } as never, { status: 400 } as never, 'quota')
		)
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		// Nothing more for this page, marks untouched by the follow-up flush.
		await putSession({ ...s, summary: 'changed' })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(await pendingDirty()).toEqual(['s4'])
	})
})

describe('sessionMirror restore', () => {
	const backup = {
		id: 's9',
		head: { id: 's9', workspace_id: 'ws', createdAt: 5, chatId: 'c9', summary: 'remote' },
		chats: [
			{
				id: 'c9',
				record: {
					id: 'c9',
					sessionId: 's9',
					title: 't',
					lastModified: 7,
					actualMessages: [],
					displayMessages: [{ role: 'user', content: 'hi' }]
				}
			}
		],
		images: []
	}

	it('brings back a session the browser lacks, and records nothing for one whose chats could not be written', async () => {
		listMock.mockResolvedValue({
			enabled: true,
			sessions: [{ id: 's9', updated_at: '2026-09-14T00:00:00Z' }]
		})
		pullMock.mockResolvedValue({ enabled: true, sessions: [backup], deferred: [] })
		usersWorkspaceStore.set({ email: EMAIL, workspaces: [] } as never)

		// A removal pending for another workspace (the session moved here from it) does
		// not stand in the way of restoring this workspace's copy.
		localStorage.setItem(`${PENDING_PREFIX}r::s9::elsewhere`, '1')
		chatImport.unavailable = true
		restoreSessionBackups('ws')
		await __settleForTesting()
		expect(pullMock).toHaveBeenCalledTimes(1)
		expect(sessionState.sessions.map((s) => s.id)).toEqual([])
		// Not recorded as restored: the next restore tries again, and no flush can push a
		// transcript-less copy over the backup.
		__resetMirrorForTesting()
		chatImport.unavailable = false
		restoreSessionBackups('ws')
		await __settleForTesting()
		expect(pullMock).toHaveBeenCalledTimes(2)
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['s9']))
		const restored = sessionState.sessions[0]
		expect(restored.name).toBe('session-1')
		expect(restored.summary).toBe('remote')
		expect(restored.lastSeenCount).toBe(1)
		expect((await readStoredChat('c9', EMAIL))?.displayMessages).toHaveLength(1)

		// Restored state is what the backup holds: nothing to push (the other workspace's
		// removal is its own request, not part of this check).
		localStorage.removeItem(`${PENDING_PREFIX}r::s9::elsewhere`)
		await __flushForTesting()
		expect(pushMock).not.toHaveBeenCalled()
	})

	it('imports a session that came in pages only once the last page arrived', async () => {
		listMock.mockResolvedValue({
			enabled: true,
			sessions: [{ id: 's9', updated_at: '2026-09-14T00:00:00Z' }]
		})
		const cursor = { id: 's9', images: false, after: 'sessions/s9/chats/c9.json' }
		const c9b = { ...backup.chats[0], id: 'c9b', record: { ...backup.chats[0].record, id: 'c9b' } }
		pullMock
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, next: cursor }],
				deferred: []
			})
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, chats: [c9b] }],
				deferred: []
			})
		usersWorkspaceStore.set({ email: EMAIL, workspaces: [] } as never)
		restoreSessionBackups('ws')
		await __settleForTesting()
		expect(pullMock).toHaveBeenCalledTimes(2)
		expect(pullMock.mock.calls[1][0].requestBody).toEqual({ ids: ['s9'], resume: cursor })
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['s9']))
		expect((await readStoredChat('c9', EMAIL))?.displayMessages).toHaveLength(1)
		expect((await readStoredChat('c9b', EMAIL))?.id).toBe('c9b')
	})

	it('starts a session over when its backup moved between two pages', async () => {
		listMock.mockResolvedValue({
			enabled: true,
			sessions: [{ id: 's9', updated_at: '2026-09-14T00:00:00Z' }]
		})
		const cursor = { id: 's9', images: false, after: 'sessions/s9/chats/c9.json' }
		const c9b = { ...backup.chats[0], id: 'c9b', record: { ...backup.chats[0].record, id: 'c9b' } }
		const c9a = { ...backup.chats[0], id: 'c9a', record: { ...backup.chats[0].record, id: 'c9a' } }
		pullMock
			// The first attempt: a chat sorting before the cursor lands between the pages.
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, next: cursor, listing: 'L1' }],
				deferred: []
			})
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, chats: [c9b], listing: 'L2' }],
				deferred: []
			})
			// The second attempt sees the whole of it.
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, chats: [c9a, backup.chats[0]], next: cursor, listing: 'L2' }],
				deferred: []
			})
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...backup, chats: [c9b], listing: 'L2' }],
				deferred: []
			})
		usersWorkspaceStore.set({ email: EMAIL, workspaces: [] } as never)
		restoreSessionBackups('ws')
		await __settleForTesting()
		expect(pullMock).toHaveBeenCalledTimes(4)
		expect(pullMock.mock.calls[2][0].requestBody).toEqual({ ids: ['s9'], resume: undefined })
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['s9']))
		expect((await readStoredChat('c9a', EMAIL))?.id).toBe('c9a')
		expect((await readStoredChat('c9b', EMAIL))?.id).toBe('c9b')
	})

	it('leaves a session another tab imported meanwhile alone, without Web Locks', async () => {
		listMock.mockResolvedValue({
			enabled: true,
			sessions: [{ id: 's9', updated_at: '2026-09-14T00:00:00Z' }]
		})
		// An earlier restore of this tab was cut short after staging a chat the backup has
		// since dropped.
		const { importStoredChats } = await import('../copilot/chat/HistoryManager.svelte')
		const cx = { ...backup.chats[0].record, id: 'cx' } as never
		await importStoredChats([cx], [], EMAIL, true)
		await __writeSyncForTesting(
			[
				{
					id: 's9',
					ws: 'ws',
					head: '',
					chats: {},
					images: {},
					staging: { chats: ['cx'], images: [], items: [], versions: [] }
				}
			],
			EMAIL
		)
		let release!: (value: unknown) => void
		pullMock.mockImplementationOnce(() => new Promise((r) => (release = r)))
		usersWorkspaceStore.set({ email: EMAIL, workspaces: [] } as never)
		restoreSessionBackups('ws')
		await vi.waitFor(() => expect(pullMock).toHaveBeenCalledTimes(1))
		// The other tab's restore lands first, with a newer transcript that has that chat.
		await importStoredChats(
			[{ ...backup.chats[0].record, title: 'newer' } as never, cx],
			[],
			EMAIL,
			true
		)
		await importSessions([{ ...backup.head, name: '' } as never], EMAIL)
		release({ enabled: true, sessions: [backup], deferred: [] })
		await __settleForTesting()
		expect((await readStoredChat('c9', EMAIL))?.title).toBe('newer')
		// Not this tab's session to prune: the staged chat the other tab holds stays.
		expect((await readStoredChat('cx', EMAIL))?.id).toBe('cx')
	})

	it('never writes an older record over a newer one, and prunes only what it is told', async () => {
		const { importStoredChats, pruneSessionChats } = await import(
			'../copilot/chat/HistoryManager.svelte'
		)
		const chat = (lastModified: number, title: string) =>
			({ ...backup.chats[0].record, lastModified, title }) as never
		await importStoredChats([chat(20, 'newer')], [], EMAIL, true)
		await importStoredChats([chat(10, 'older')], [], EMAIL, true)
		expect((await readStoredChat('c9', EMAIL))?.title).toBe('newer')
		await importStoredChats(
			[{ ...backup.chats[0].record, id: 'c9b', lastModified: 30 } as never],
			[],
			EMAIL,
			true
		)
		// A prune names what goes; nothing else of the session is touched.
		await pruneSessionChats('s9', new Set(['c9']), new Set(), EMAIL)
		expect(await readStoredChat('c9', EMAIL)).toBeUndefined()
		expect((await readStoredChat('c9b', EMAIL))?.id).toBe('c9b')
	})

	it('keeps an image whose chat came on an earlier page, and restages after a page failed', async () => {
		listMock.mockResolvedValue({
			enabled: true,
			sessions: [{ id: 's9', updated_at: '2026-09-14T00:00:00Z' }]
		})
		const cursor = { id: 's9', images: true, after: '' }
		const imagePage = {
			...backup,
			chats: [],
			images: [{ chat_id: 'c9', id: 'i9', data_url: IMAGE }]
		}
		// The first restore is cut short after staging the chats and an artifact with two
		// versions.
		const gone = { ...backup.chats[0], id: 'cx', record: { ...backup.chats[0].record, id: 'cx' } }
		const item = { id: 'a1', sessionId: 's9', kind: 'markdown', name: 'a', content: 'x' }
		const version = (n: number) => ({
			key: `a1:${n}`,
			artifactId: 'a1',
			version: n,
			name: 'a',
			content: 'x',
			savedAt: n
		})
		pullMock
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [
					{
						...backup,
						chats: [...backup.chats, gone],
						artifacts: { items: [item], versions: [version(1), version(2)] },
						next: cursor
					}
				],
				deferred: []
			})
			.mockRejectedValueOnce(new Error('offline'))
		usersWorkspaceStore.set({ email: EMAIL, workspaces: [] } as never)
		restoreSessionBackups('ws')
		await __settleForTesting()
		expect(pullMock).toHaveBeenCalledTimes(2)
		expect(sessionState.sessions.map((s) => s.id)).toEqual([])
		expect((await readStoredChat('c9', EMAIL))?.title).toBe('t')
		expect((await readStoredChat('cx', EMAIL))?.id).toBe('cx')

		// The backup moved on meanwhile: the retry takes the newer chat over the staged one
		// and drops the chat the backup no longer has.
		__resetMirrorForTesting()
		const newer = {
			...backup,
			chats: [{ ...backup.chats[0], record: { ...backup.chats[0].record, title: 'newer' } }],
			artifacts: { items: [item], versions: [version(1)] }
		}
		pullMock
			.mockResolvedValueOnce({
				enabled: true,
				sessions: [{ ...newer, next: cursor }],
				deferred: []
			})
			.mockResolvedValueOnce({ enabled: true, sessions: [imagePage], deferred: [] })
		restoreSessionBackups('ws')
		await __settleForTesting()
		await vi.waitFor(() => expect(sessionState.sessions.map((s) => s.id)).toEqual(['s9']))
		expect((await readStoredChat('c9', EMAIL))?.title).toBe('newer')
		expect(await readStoredChat('cx', EMAIL)).toBeUndefined()
		const { readImageDataUrl } = await import('../copilot/chat/HistoryManager.svelte')
		expect(await readImageDataUrl('i9', EMAIL)).toBe(IMAGE)
		// The version the backup no longer has went with the chat it no longer has.
		const { readSessionArtifacts } = await import('../copilot/chat/artifacts/artifactsDB')
		const artifacts = await readSessionArtifacts('s9', EMAIL)
		expect(artifacts?.items.map((i) => i.id)).toEqual(['a1'])
		expect(artifacts?.versions.map((v) => v.key)).toEqual(['a1:1'])
	})
})
