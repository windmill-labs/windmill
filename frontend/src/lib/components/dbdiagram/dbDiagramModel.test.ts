import { describe, expect, it } from 'vitest'
import { CARD_WIDTH, cardHeight, layoutTables, type DiagramTable } from './dbDiagramModel'

function table(name: string, columnCount: number): DiagramTable {
	return {
		key: `shop.${name}`,
		schema: 'shop',
		table: name,
		columns: Array.from({ length: columnCount }, (_, i) => ({
			name: `c${i}`,
			datatype: 'text',
			isPrimaryKey: i === 0
		}))
	}
}

describe('layoutTables', () => {
	// Runs again on every checkbox click, so cards must not land on top of each
	// other for any mix of card heights.
	it('never overlaps two cards', () => {
		const tables = Array.from({ length: 40 }, (_, i) => table(`t${i}`, 1 + (i % 17)))
		const expanded = new Set(['shop.t3', 'shop.t20'])
		const positions = layoutTables(tables, expanded)

		const boxes = tables.map((t) => ({
			x: positions[t.key].x,
			y: positions[t.key].y,
			h: cardHeight(t, expanded.has(t.key))
		}))
		for (let i = 0; i < boxes.length; i++) {
			for (let j = i + 1; j < boxes.length; j++) {
				const a = boxes[i]
				const b = boxes[j]
				const apart =
					a.x + CARD_WIDTH <= b.x || b.x + CARD_WIDTH <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y
				expect(apart, `${tables[i].key} overlaps ${tables[j].key}`).toBe(true)
			}
		}
	})

	it('places the same tables in the same spots every time', () => {
		const tables = Array.from({ length: 12 }, (_, i) => table(`t${i}`, 3 + i))
		expect(layoutTables(tables, new Set())).toEqual(layoutTables(tables, new Set()))
	})
})
