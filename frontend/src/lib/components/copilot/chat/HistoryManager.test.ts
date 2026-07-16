import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { openDB } from 'idb'

// scopedKey resolves the email from userStore via a BROWSER-gated subscription.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

import { userStore, type UserExt } from '$lib/stores'
import HistoryManager, { __resetLegacyChatClaimForTesting } from './HistoryManager.svelte'
import type { DisplayMessage } from './shared'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

function asUser(email: string): UserExt {
	return { email, username: email.split('@')[0] } as unknown as UserExt
}

type LegacyChat = {
	id: string
	actualMessages: unknown[]
	displayMessages: unknown[]
	title: string
	lastModified: number
	sessionId?: string
}

async function seedLegacyChatDb(records: LegacyChat[]) {
	const db = await openDB('copilot-chat-history', 1, {
		upgrade(d) {
			if (!d.objectStoreNames.contains('chats')) d.createObjectStore('chats', { keyPath: 'id' })
		}
	})
	for (const r of records) await db.put('chats' as never, r as never)
	db.close()
}

beforeEach(() => {
	;(globalThis as any).indexedDB = new IDBFactory()
	__resetLegacyChatClaimForTesting()
	userStore.set(asUser('admin@test'))
})

async function countChats(dbName: string): Promise<number> {
	const db = await openDB(dbName)
	if (!db.objectStoreNames.contains('chats')) {
		db.close()
		return 0
	}
	const n = await db.count('chats' as never)
	db.close()
	return n
}

describe('HistoryManager legacy chat-history migration', () => {
	it('claims the legacy un-namespaced DB into the per-user DB, then deletes it', async () => {
		await seedLegacyChatDb([
			{ id: 'c1', actualMessages: [], displayMessages: [], title: 'Old chat', lastModified: 1 },
			{
				id: 'c2',
				actualMessages: [],
				displayMessages: [],
				title: 'Tagged chat',
				lastModified: 2,
				sessionId: 's9'
			}
		])

		const hm = new HistoryManager()
		await hm.init()

		// Both session-tagged and untagged legacy chats are claimed.
		expect(
			hm
				.getAllSavedChats()
				.map((c) => c.id)
				.sort()
		).toEqual(['c1', 'c2'])

		const names = (await indexedDB.databases()).map((d) => d.name)
		expect(names).toContain('copilot-chat-history::admin@test')
		// Bare legacy DB is gone so a later different user does not re-claim it.
		expect(names).not.toContain('copilot-chat-history')
	})

	it('starts empty and does not throw when there is no legacy DB', async () => {
		const hm = new HistoryManager()
		await hm.init()
		expect(hm.getAllSavedChats()).toEqual([])
	})

	it('serves a persisted tool image to a fresh instance (reload survival)', async () => {
		const hm = new HistoryManager()
		await hm.init()
		await hm.saveToolImage('tool-1', 'data:image/png;base64,FULL')

		const reloaded = new HistoryManager()
		await reloaded.init()
		await expect(reloaded.loadToolImage('tool-1')).resolves.toBe('data:image/png;base64,FULL')
		await expect(reloaded.loadToolImage('unknown')).resolves.toBeUndefined()
	})

	it('caps stored tool images per chat, evicting the oldest', async () => {
		const hm = new HistoryManager()
		await hm.init()
		for (let i = 0; i <= 30; i++) {
			await hm.saveToolImage(`tool-${String(i).padStart(2, '0')}`, `data:${i}`)
		}
		await expect(hm.loadToolImage('tool-00')).resolves.toBeUndefined()
		await expect(hm.loadToolImage('tool-01')).resolves.toBe('data:1')
		await expect(hm.loadToolImage('tool-30')).resolves.toBe('data:30')
	})

	it("deletes a chat's tool images along with the chat", async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		await hm.saveToolImage('tool-1', 'data:full')
		await hm.saveChat(
			[{ role: 'user', content: 'x' }] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)

		hm.deletePastChat(chatId)

		await vi.waitFor(async () => {
			expect(await hm.loadToolImage('tool-1')).toBeUndefined()
		})
	})

	it('writes land in the current user DB after an in-place user switch', async () => {
		const hm = new HistoryManager()
		await hm.init()

		// Save under user A.
		await hm.save(
			[{ role: 'user', content: 'hello A' }] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		expect(await countChats('copilot-chat-history::admin@test')).toBe(1)

		// Switch identity in-place (no reload), then save again. The write must go
		// to user B's DB, not A's stale handle.
		userStore.set(asUser('other@test'))
		await hm.save(
			[{ role: 'user', content: 'hello B' }] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)

		expect(await countChats('copilot-chat-history::other@test')).toBe(1)
		// A's DB is untouched by the post-switch write.
		expect(await countChats('copilot-chat-history::admin@test')).toBe(1)
	})
})

describe('HistoryManager title across compaction', () => {
	it('keeps the original title once a summary boundary leads the transcript', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const id = hm.getCurrentChatId()

		// First save derives the title from the first user message.
		await hm.save(
			[{ role: 'user', content: 'original first question', index: 0 }] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		expect(hm.getAllSavedChats().find((c) => c.id === id)?.title).toBe('original first question')

		// After compaction the transcript leads with a summary boundary; deriving
		// the title now would shift it to the surviving tail message. It must stay
		// the title computed before compaction.
		await hm.save(
			[
				{ role: 'summary', content: 'summary of the earlier conversation' },
				{ role: 'user', content: 'a much later follow-up', index: 1 }
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		expect(hm.getAllSavedChats().find((c) => c.id === id)?.title).toBe('original first question')
	})
})

describe('HistoryManager modified-items mask persistence', () => {
	const msgs = [{ role: 'user', content: 'hello', index: 0 }] as DisplayMessage[]

	it('a save without the argument preserves a previously stored mask', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const id = hm.getCurrentChatId()

		await hm.saveChat(msgs, [] as ChatCompletionMessageParam[], undefined, ['script:u/a/x'])
		expect(hm.getModifiedItems(id)).toEqual(['script:u/a/x'])

		// e.g. manual compaction re-saving the transcript: the whole record is
		// rewritten, but the tracked mask must survive.
		await hm.saveChat(msgs, [] as ChatCompletionMessageParam[])
		expect(hm.getModifiedItems(id)).toEqual(['script:u/a/x'])
	})

	it('never retroactively stamps an untracked chat', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const id = hm.getCurrentChatId()

		await hm.saveChat(msgs, [] as ChatCompletionMessageParam[])
		expect(hm.getModifiedItems(id)).toBeUndefined()
	})
})
