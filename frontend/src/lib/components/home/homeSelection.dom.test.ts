import { describe, it, expect, beforeEach } from 'vitest'
import { HomeSelection, type BulkItem } from './homeSelection.svelte'

// A shift-click range resolves its ends through the rendered rows, so these need a
// document: the keys come back from `[data-row-selection-key]` in visual order.
function row(key: string): BulkItem {
	return {
		key,
		kind: 'script',
		path: key.slice('script/'.length),
		displayPath: key.slice('script/'.length),
		summary: '',
		canWrite: true,
		owner: true,
		archived: false,
		draftOnly: false,
		isDraft: false,
		rawApp: false
	}
}

const a = row('script/f/a/one')
const b = row('script/f/a/two')
const c = row('script/f/a/three')

function rendered(...items: BulkItem[]): HomeSelection {
	document.body.innerHTML = items
		.map((i) => `<div data-row-selection-key="${i.key}"></div>`)
		.join('')
	const s = new HomeSelection()
	s.available = true
	for (const i of items) s.register(i)
	return s
}

describe('HomeSelection shift-range', () => {
	beforeEach(() => {
		document.body.innerHTML = ''
	})

	it('selects the span between the anchor and the clicked row', () => {
		const s = rendered(a, b, c)
		s.toggle(a)

		s.toggle(c, true)

		expect(s.items.map((i) => i.key)).toEqual([a.key, b.key, c.key])
	})

	it('does not reach back into a selection the user already emptied', () => {
		const s = rendered(a, b, c)
		s.toggle(a)
		s.toggle(a)

		s.toggle(c, true)

		expect(s.items.map((i) => i.key)).toEqual([c.key])
	})
})
