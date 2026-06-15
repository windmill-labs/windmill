import { describe, it, expect, beforeEach, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { openDB } from 'idb'

// scopedKey resolves the email from userStore via a BROWSER-gated subscription.
vi.mock('esm-env', async (importOriginal) => ({
	...(await importOriginal<typeof import('esm-env')>()),
	BROWSER: true
}))

import { userStore, type UserExt } from '$lib/stores'
import HistoryManager from './HistoryManager.svelte'

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
	userStore.set(asUser('admin@test'))
})

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
})
