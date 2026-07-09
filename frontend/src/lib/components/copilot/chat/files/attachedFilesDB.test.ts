import { describe, expect, it } from 'vitest'
import {
	getItemsForSession,
	putItem,
	deleteItem,
	deleteItemsForSession,
	ensurePersistentStorage
} from './attachedFilesDB'

// IndexedDB is unavailable in the node test env. The module must degrade gracefully
// (open fails → reads return [], writes/deletes are no-ops) rather than throwing.
describe('attachedFilesDB without IndexedDB', () => {
	it('returns [] for reads', async () => {
		expect(await getItemsForSession('s1')).toEqual([])
	})

	it('does not throw on writes/deletes', async () => {
		await expect(
			putItem({ id: 'a', sessionId: 's1', kind: 'snapshot', name: 'x.txt', addedAt: 0 })
		).resolves.toBeUndefined()
		await expect(deleteItem('a')).resolves.toBeUndefined()
		await expect(deleteItemsForSession('s1')).resolves.toBeUndefined()
	})

	it('does not throw when requesting persistent storage', async () => {
		await expect(ensurePersistentStorage()).resolves.toBeUndefined()
	})
})
