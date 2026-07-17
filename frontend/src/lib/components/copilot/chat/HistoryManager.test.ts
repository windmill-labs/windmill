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

	it('persists image bytes out of the chat record and hydrates them back on load', async () => {
		const png = 'data:image/png;base64,FULLBYTES'
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		await hm.saveChat(
			[
				{ role: 'user', content: 'look', images: [{ dataUrl: png, mediaType: 'image/png' }] },
				{ role: 'tool', tool_call_id: 't1', content: 'shot', imageUrl: png }
			] as DisplayMessage[],
			[
				{
					role: 'user',
					content: [
						{ type: 'text', text: 'look' },
						{ type: 'image_url', image_url: { url: png } }
					]
				}
			] as ChatCompletionMessageParam[]
		)

		// The chat record holds refs, not bytes — and the shared data URL of the
		// bubble, tool card, and API part dedups to a single blob record.
		const db = await openDB('copilot-chat-history::admin@test')
		const record = await db.get('chats' as never, chatId)
		expect(JSON.stringify(record)).not.toContain('FULLBYTES')
		expect((record as any).actualMessages[0].content[1].image_url.url).toMatch(/^wm-image:/)
		expect(await db.count('images' as never)).toBe(1)
		db.close()

		// A fresh instance (reload) hydrates the refs back to the original bytes.
		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.actualMessages[0].content as any[])[1].image_url.url).toBe(png)
		expect((chat?.displayMessages[0] as any).images[0].dataUrl).toBe(png)
		expect((chat?.displayMessages[1] as any).imageUrl).toBe(png)
	})

	it('re-saving the same conversation does not mint new blob records', async () => {
		const png = 'data:image/png;base64,STABLE'
		const display = [
			{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
		] as DisplayMessage[]
		const hm = new HistoryManager()
		await hm.init()
		await hm.saveChat(display, [] as ChatCompletionMessageParam[])
		await hm.saveChat(display, [] as ChatCompletionMessageParam[])

		const db = await openDB('copilot-chat-history::admin@test')
		expect(await db.count('images' as never)).toBe(1)
		db.close()
	})

	it('caps stored blobs per chat; an evicted ref hydrates to the omitted placeholder', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const urlFor = (i: number) => `data:image/png;base64,IMG${String(i).padStart(2, '0')}`
		const messages = [] as ChatCompletionMessageParam[]
		for (let i = 0; i <= 30; i++) {
			messages.push({
				role: 'user',
				content: [{ type: 'image_url', image_url: { url: urlFor(i) } }]
			} as ChatCompletionMessageParam)
			await hm.saveChat([{ role: 'user', content: 'x' }] as DisplayMessage[], messages)
		}

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.actualMessages[0].content as any[])[0]).toEqual({
			type: 'text',
			text: '[image omitted]'
		})
		expect((chat?.actualMessages[1].content as any[])[0].image_url.url).toBe(urlFor(1))
		expect((chat?.actualMessages[30].content as any[])[0].image_url.url).toBe(urlFor(30))
	})

	it("deletes a chat's image blobs along with the chat", async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		await hm.saveChat(
			[
				{
					role: 'user',
					content: 'x',
					images: [{ dataUrl: 'data:image/png;base64,GONE', mediaType: 'image/png' }]
				}
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)

		hm.deletePastChat(chatId)

		await vi.waitFor(async () => {
			const db = await openDB('copilot-chat-history::admin@test')
			const count = await db.count('images' as never)
			db.close()
			expect(count).toBe(0)
		})
	})

	it('loads pre-blob-store records with inline data URLs untouched', async () => {
		const png = 'data:image/png;base64,LEGACYINLINE'
		const hm = new HistoryManager()
		await hm.init()
		// Simulate a record persisted before the blob store existed.
		const db = await openDB('copilot-chat-history::admin@test')
		await db.put(
			'chats' as never,
			{
				id: 'legacy1',
				title: 'legacy',
				lastModified: 1,
				actualMessages: [
					{ role: 'user', content: [{ type: 'image_url', image_url: { url: png } }] }
				],
				displayMessages: [
					{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
				]
			} as never
		)
		db.close()

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat('legacy1')
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(png)
		expect((chat?.displayMessages[0] as any).images[0].dataUrl).toBe(png)
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
