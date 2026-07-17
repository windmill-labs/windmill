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
		// Re-saving the over-cap chat must not resurrect the evicted oldest blob
		// (its live data URL is still in the arrays): a re-put would stamp it
		// newest and push the eviction onto a newer image, and repeated saves
		// would rotate the hole toward the latest attachment.
		await hm.saveChat([{ role: 'user', content: 'x' }] as DisplayMessage[], messages)
		await hm.saveChat([{ role: 'user', content: 'x' }] as DisplayMessage[], messages)

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

	it('keeps blob chronology when drop-oldest compaction removed old API counterparts', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const urlFor = (i: number) => `data:image/png;base64,IMG${String(i).padStart(2, '0')}`
		// Transcript keeps all 31 bubbles; drop-oldest compaction pruned the API
		// history down to the newest 4 image messages.
		const display = Array.from({ length: 31 }, (_, i) => ({
			role: 'user',
			content: 'x',
			index: i - 27,
			images: [{ dataUrl: urlFor(i), mediaType: 'image/png' }]
		})) as DisplayMessage[]
		const messages = Array.from({ length: 4 }, (_, i) => ({
			role: 'user',
			content: [{ type: 'image_url', image_url: { url: urlFor(27 + i) } }]
		})) as ChatCompletionMessageParam[]
		await hm.saveChat(display, messages)
		await hm.saveChat(display, messages)

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		// The oldest transcript image is the one over the cap...
		expect((chat?.displayMessages[0] as any).images).toBeUndefined()
		// ...never a newer one that merely lost its API counterpart ordering.
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(urlFor(27))
		expect((chat?.displayMessages[30] as any).images[0].dataUrl).toBe(urlFor(30))
	})

	it('truncated turns release their blobs from the cap', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const urlFor = (i: number) => `data:image/png;base64,IMG${String(i).padStart(2, '0')}`
		const imageMsg = (i: number) =>
			({
				role: 'user',
				content: [{ type: 'image_url', image_url: { url: urlFor(i) } }]
			}) as ChatCompletionMessageParam
		const display = [{ role: 'user', content: 'x' }] as DisplayMessage[]
		// Fill the cap exactly, then retry/edit truncates the tail to 5 messages
		// and adds one replacement image. The truncated turns' blobs must stop
		// counting against the cap — image 0 is among the newest 6 *referenced*
		// images and must survive.
		await hm.saveChat(
			display,
			Array.from({ length: 30 }, (_, i) => imageMsg(i))
		)
		await hm.saveChat(display, [...Array.from({ length: 5 }, (_, i) => imageMsg(i)), imageMsg(99)])

		const db = await openDB('copilot-chat-history::admin@test')
		expect(await db.count('images' as never)).toBe(6)
		db.close()

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(urlFor(0))
		expect((chat?.actualMessages[5].content as any[])[0].image_url.url).toBe(urlFor(99))
	})

	it('re-attaching identical bytes ranks the image by its newest reference', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const urlFor = (i: number) => `data:image/png;base64,IMG${String(i).padStart(2, '0')}`
		const imageMsg = (url: string) =>
			({
				role: 'user',
				content: [{ type: 'image_url', image_url: { url } }]
			}) as ChatCompletionMessageParam
		const display = [{ role: 'user', content: 'x' }] as DisplayMessage[]
		const reused = 'data:image/png;base64,REUSED'
		// The reused image appears first, 30 distinct images follow, then it is
		// attached again. Its newest reference makes it one of the newest 30
		// distinct images, so the eviction must land on the oldest of the middle
		// ones — not on the image the user just re-attached.
		const messages = [
			imageMsg(reused),
			...Array.from({ length: 30 }, (_, i) => imageMsg(urlFor(i))),
			imageMsg(reused)
		]
		await hm.saveChat(display, [messages[0]])
		await hm.saveChat(display, messages)

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(reused)
		expect((chat?.actualMessages[31].content as any[])[0].image_url.url).toBe(reused)
		expect((chat?.actualMessages[1].content as any[])[0]).toEqual({
			type: 'text',
			text: '[image omitted]'
		})
		expect((chat?.actualMessages[2].content as any[])[0].image_url.url).toBe(urlFor(1))
	})

	it('a save overlapping an older save keeps every blob its record references', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const X = 'data:image/png;base64,XBYTES'
		const Y = 'data:image/png;base64,YBYTES'
		const display = [{ role: 'user', content: 'x' }] as DisplayMessage[]
		const msg = (url: string) =>
			({
				role: 'user',
				content: [{ type: 'image_url', image_url: { url } }]
			}) as ChatCompletionMessageParam
		await hm.saveChat(display, [msg(X)])

		// An overlapping pair: the older snapshot no longer references X (retry
		// truncation), the newer one re-references it (as its newest image) and
		// adds Y. Un-serialized, the older save's delete pass removes X's blob
		// after the newer save verified its existence and moved on, landing the
		// winning record with a dangling ref.
		const older = hm.saveChat(display, [
			{ role: 'user', content: 'no images' } as ChatCompletionMessageParam
		])
		const newer = hm.saveChat(display, [msg(Y), msg(X)])
		await Promise.all([older, newer])

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(Y)
		expect((chat?.actualMessages[1].content as any[])[0].image_url.url).toBe(X)
	})

	it('reopening a rotated chat reuses its blob records on the next save', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const png = 'data:image/png;base64,STABLEBYTES'
		await hm.save(
			[
				{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		) // rotates to a fresh chat, pruning the id cache

		const before = await openDB('copilot-chat-history::admin@test')
		const idsBefore = await before.getAllKeys('images' as never)
		before.close()

		// Reopening must reseed the stable blob id — a re-save that minted a new
		// id would rewrite every blob (and delete the old ones) on each reopen.
		const chat = await hm.loadPastChat(chatId)
		await hm.saveChat(chat!.displayMessages as DisplayMessage[], chat!.actualMessages)

		const after = await openDB('copilot-chat-history::admin@test')
		const idsAfter = await after.getAllKeys('images' as never)
		after.close()
		expect(idsAfter).toEqual(idsBefore)
	})

	it('the same image in two chats gets two owned blobs; deleting one chat spares the other', async () => {
		const png = 'data:image/png;base64,SHAREDBYTES'
		const display = [
			{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
		] as DisplayMessage[]
		const hm = new HistoryManager()
		await hm.init()
		const chatA = hm.getCurrentChatId()
		await hm.save(display, [] as ChatCompletionMessageParam[]) // rotates to a new chat
		const chatB = hm.getCurrentChatId()
		await hm.saveChat(display, [] as ChatCompletionMessageParam[])

		const db = await openDB('copilot-chat-history::admin@test')
		expect(await db.count('images' as never)).toBe(2)
		db.close()

		hm.deletePastChat(chatA)
		await vi.waitFor(async () => {
			const d = await openDB('copilot-chat-history::admin@test')
			const count = await d.count('images' as never)
			d.close()
			expect(count).toBe(1)
		})

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatB)
		expect((chat?.displayMessages[0] as any).images[0].dataUrl).toBe(png)
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

	it("a failed record put cannot orphan the previous record's blobs", async () => {
		const png = 'data:image/png;base64,SURVIVES'
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		await hm.saveChat(
			[
				{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)

		// Make the next `chats` put fail (quota/connection failure), on a save
		// whose record drops the image — its blob is now stale, but deleting it
		// before the record commit would corrupt the still-current OLD record.
		const probe = await openDB('probe-proto', 1, {
			upgrade: (d) => {
				d.createObjectStore('s')
			}
		})
		const proto = Object.getPrototypeOf(
			probe.transaction('s' as never, 'readwrite').objectStore('s' as never)
		)
		probe.close()
		const origPut = proto.put
		let failNext = true
		proto.put = function (this: { name: string }, ...args: unknown[]) {
			if (this.name === 'chats' && failNext) {
				failNext = false
				throw new Error('simulated quota failure')
			}
			return origPut.apply(this, args)
		}
		try {
			await expect(
				hm.saveChat(
					[{ role: 'user', content: 'no image' }] as DisplayMessage[],
					[] as ChatCompletionMessageParam[]
				)
			).rejects.toThrow('simulated quota failure')
		} finally {
			proto.put = origPut
		}

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.displayMessages[0] as any).images[0].dataUrl).toBe(png)
	})

	it('drops queued writes when the user switches before they execute (no cross-user leak)', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const display = [
			{
				role: 'user',
				content: 'private to A',
				images: [{ dataUrl: 'data:image/png;base64,LEAKBYTES', mediaType: 'image/png' }]
			}
		] as DisplayMessage[]
		// Enqueue two writes under user A, then switch identity before either
		// executes. Resolving the DB handle at execution time would write A's
		// chat and image blob into B's database.
		const first = hm.saveChat(display, [] as ChatCompletionMessageParam[])
		const second = hm.saveChat(display, [] as ChatCompletionMessageParam[])
		userStore.set(asUser('other@test'))
		await Promise.all([first, second])

		const db = await openDB('copilot-chat-history::other@test')
		const chats = db.objectStoreNames.contains('chats') ? await db.count('chats' as never) : 0
		const images = db.objectStoreNames.contains('images') ? await db.count('images' as never) : 0
		db.close()
		expect(chats).toBe(0)
		expect(images).toBe(0)
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

describe('HistoryManager image-only chats', () => {
	it('titles an image-only chat from its attachment instead of leaving it blank', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const id = hm.getCurrentChatId()
		await hm.saveChat(
			[
				{
					role: 'user',
					content: '',
					images: [
						{ dataUrl: 'data:image/png;base64,A', mediaType: 'image/png', name: 'mockup.png' }
					]
				}
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		expect(hm.getAllSavedChats().find((c) => c.id === id)?.title).toBe('mockup.png')
	})

	it('keeps the filename title when the evicted bubble re-saves as an omission marker', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const id = hm.getCurrentChatId()
		await hm.saveChat(
			[
				{
					role: 'user',
					content: '',
					images: [
						{ dataUrl: 'data:image/png;base64,A', mediaType: 'image/png', name: 'mockup.png' }
					]
				}
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		// Post-reload shape of an evicted image-only first bubble: the omission
		// marker as content, images gone. Re-saving must not adopt the marker as
		// the chat's title.
		await hm.saveChat(
			[
				{ role: 'user', content: '[image omitted]' },
				{ role: 'user', content: 'follow-up' }
			] as DisplayMessage[],
			[] as ChatCompletionMessageParam[]
		)
		expect(hm.getAllSavedChats().find((c) => c.id === id)?.title).toBe('mockup.png')
	})

	it('shows an omission marker when an evicted image-only bubble reloads', async () => {
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const urlFor = (i: number) => `data:image/png;base64,IMG${String(i).padStart(2, '0')}`
		// 31 image-only turns: the oldest exceeds the blob cap, so its bubble
		// reloads with no image AND no text — it must say what happened instead
		// of rendering empty.
		const display = Array.from({ length: 31 }, (_, i) => ({
			role: 'user',
			content: '',
			index: i,
			images: [{ dataUrl: urlFor(i), mediaType: 'image/png' }]
		})) as DisplayMessage[]
		await hm.saveChat(display, [] as ChatCompletionMessageParam[])

		const reloaded = new HistoryManager()
		await reloaded.init()
		const chat = await reloaded.loadPastChat(chatId)
		expect((chat?.displayMessages[0] as any).images).toBeUndefined()
		expect((chat?.displayMessages[0] as any).content).toBe('[image omitted]')
		expect((chat?.displayMessages[1] as any).images[0].dataUrl).toBe(urlFor(1))
		expect((chat?.displayMessages[1] as any).content).toBe('')
	})
})

describe('HistoryManager without IndexedDB', () => {
	it('keeps history usable from memory when the database is unavailable', async () => {
		// whenReady() resolves undefined when opens fail (private browsing,
		// blocked, corrupt) — history then lives in memory only, and must keep
		// its inline image bytes: refs without a backing store would break
		// thumbnails and resend `wm-image:` URLs on retry.
		;(globalThis as any).indexedDB = {
			open: () => {
				throw new Error('blocked')
			}
		}
		const hm = new HistoryManager()
		await hm.init()
		const chatId = hm.getCurrentChatId()
		const png = 'data:image/png;base64,MEMORYONLY'
		await hm.saveChat(
			[
				{ role: 'user', content: 'x', images: [{ dataUrl: png, mediaType: 'image/png' }] }
			] as DisplayMessage[],
			[
				{
					role: 'user',
					content: [{ type: 'image_url', image_url: { url: png } }]
				}
			] as ChatCompletionMessageParam[]
		)

		const chat = await hm.loadPastChat(chatId)
		expect((chat?.displayMessages[0] as any).images[0].dataUrl).toBe(png)
		expect((chat?.actualMessages[0].content as any[])[0].image_url.url).toBe(png)
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
