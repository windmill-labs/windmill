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
	__resetLegacyChatClaimForTesting,
	readStoredChat
} from '../copilot/chat/HistoryManager.svelte'
import { deleteSession, putSession, sessionState, type Session } from './sessionState.svelte'
import {
	__flushForTesting,
	__resetMirrorForTesting,
	__settleForTesting,
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

beforeEach(async () => {
	;(globalThis as any).indexedDB = new IDBFactory()
	localStorage.clear()
	__resetLegacyChatClaimForTesting()
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
		expect(entry.head).toEqual({ id: 's1', createdAt: 1, workspace_id: 'ws', chatId: 'c1' })
		expect(entry.chats.map((c: { id: string }) => c.id)).toEqual(['c1'])
		// The record keeps its blob ref; bytes travel as the image object only.
		expect(JSON.stringify(entry.chats[0].record)).not.toContain(IMAGE)
		expect(entry.artifacts).toBeUndefined()
		expect(pendingKeys()).toEqual([])

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

	it('keeps the marks when the push fails, and stops for a workspace without storage', async () => {
		const s: Session = { id: 's2', name: 'session-2', createdAt: 1, workspace_id: 'ws' }
		sessionState.sessions = [s]
		await putSession(s)

		pushMock.mockRejectedValueOnce(new TypeError('network'))
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(1)
		expect(pendingKeys()).toEqual(['d::s2'])
		// Still marked: the retry carries it again once the backoff lapses.
		__resetMirrorForTesting()
		pushMock.mockResolvedValueOnce({ enabled: false, results: [] })
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)

		// The workspace answered that it has nowhere to keep backups: no further request,
		// and a delete there has nothing to remove, so its mark is consumed rather than
		// rescheduling a flush for the life of the tab.
		await putSession({ ...s, summary: 'changed' })
		deleteSession('s2')
		await flush()
		await __flushForTesting()
		expect(pushMock).toHaveBeenCalledTimes(2)
		expect(pendingKeys()).toEqual([])
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
		expect(pendingKeys()).toEqual(['d::s3'])
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
		expect(pendingKeys()).toEqual(['d::s4'])
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

		// Restored state is what the backup holds: nothing to push.
		await __flushForTesting()
		expect(pushMock).not.toHaveBeenCalled()
	})
})
