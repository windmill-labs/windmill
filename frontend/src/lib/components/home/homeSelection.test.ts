import { describe, it, expect } from 'vitest'
import { HomeSelection, type BulkItem } from './homeSelection.svelte'

function bulk(key: string, over: Partial<BulkItem> = {}): BulkItem {
	return {
		key,
		kind: 'script',
		path: key.split('/').slice(1).join('/'),
		displayPath: key.split('/').slice(1).join('/'),
		summary: '',
		canWrite: true,
		owner: true,
		archived: false,
		draftOnly: false,
		isDraft: false,
		rawApp: false,
		...over
	}
}

function selectionOf(...items: BulkItem[]): HomeSelection {
	const s = new HomeSelection()
	s.available = true
	for (const i of items) {
		s.register(i)
		s.toggle(i)
	}
	return s
}

// A row can be moved or deleted from its own menu while a selection is active,
// and a bulk action addressing the dead path afterwards is the failure mode.
describe('HomeSelection.dropVanished', () => {
	it('drops a selected row that was on screen before the reload and is gone after', () => {
		const a = bulk('script/f/a/one')
		const b = bulk('script/f/a/two')
		const s = selectionOf(a, b)
		const before = s.renderedKeys

		// `b` was deleted: its row unmounts and does not come back.
		s.unregister(b.key)
		s.dropVanished(before)

		expect(s.items.map((i) => i.key)).toEqual([a.key])
	})

	it('re-keys a moved row: the old key goes, the new one is a fresh selection', () => {
		const a = bulk('script/f/a/one')
		const s = selectionOf(a)
		const before = s.renderedKeys

		// A move changes the path, hence the key — the row comes back as a new one.
		s.unregister(a.key)
		s.register(bulk('script/f/b/one'))
		s.dropVanished(before)

		expect(s.items).toEqual([])
	})

	it('keeps a selected row that was never on screen', () => {
		// Selections survive narrowing the view, so a row absent from the reload is
		// not evidence that it is gone.
		const offScreen = bulk('script/f/other/one')
		const s = selectionOf(offScreen)
		s.unregister(offScreen.key)

		s.dropVanished(new Set())

		expect(s.items.map((i) => i.key)).toEqual([offScreen.key])
	})
})

describe('HomeSelection.register', () => {
	it('refreshes the snapshot of an already-selected row', () => {
		const item = bulk('script/f/a/one')
		const s = selectionOf(item)

		// The row was archived from its own menu and re-rendered.
		s.register(bulk('script/f/a/one', { archived: true }))

		expect(s.items[0].archived).toBe(true)
	})
})
