import { describe, expect, it } from 'vitest'
import { buildRelationIndex, groupForeignKeyRows, type RawAllForeignKeyRow } from './dbRelations'

function row(over: Partial<RawAllForeignKeyRow>): RawAllForeignKeyRow {
	return {
		fk_constraint_name: 'fk',
		source_schema: 'shop',
		source_table: 'orders',
		source_column: 'customer_id',
		target_schema: 'shop',
		target_table: 'customers',
		target_column: 'id',
		ordinal: 1,
		...over
	}
}

describe('groupForeignKeyRows', () => {
	it('pairs a composite key by ordinal, whatever order the rows arrive in', () => {
		const relations = groupForeignKeyRows([
			row({
				fk_constraint_name: 'returns_order_item_fkey',
				source_table: 'returns',
				source_column: 'sku',
				target_table: 'order_items',
				target_column: 'sku',
				ordinal: 2
			}),
			row({
				fk_constraint_name: 'returns_order_item_fkey',
				source_table: 'returns',
				source_column: 'order_id',
				target_table: 'order_items',
				target_column: 'order_id',
				ordinal: 1
			})
		])

		expect(relations).toHaveLength(1)
		expect(relations[0].from.columns).toEqual(['order_id', 'sku'])
		expect(relations[0].to.columns).toEqual(['order_id', 'sku'])
	})

	it('keeps same-named constraints on different tables apart', () => {
		const relations = groupForeignKeyRows([
			row({ fk_constraint_name: 'owner_fkey', source_table: 'orders' }),
			row({ fk_constraint_name: 'owner_fkey', source_table: 'addresses' })
		])
		expect(relations.map((r) => r.from.table).sort()).toEqual(['addresses', 'orders'])
	})
})

describe('buildRelationIndex', () => {
	const index = buildRelationIndex(
		groupForeignKeyRows([
			row({ source_table: 'orders', source_column: 'customer_id', target_column: 'id' }),
			row({
				fk_constraint_name: 'items_order_fkey',
				source_table: 'order_items',
				source_column: 'order_id',
				target_table: 'orders',
				target_column: 'id'
			})
		])
	)

	it('links a column only to the column it is actually paired with', () => {
		// `order_items.order_id` points at `orders.id`, not at the same-named
		// column of every other table taking part in a relation.
		expect([...(index.linkedColumns.get('shop.order_items.order_id') ?? [])]).toEqual([
			'shop.orders.id'
		])
	})

	it('relates tables in both directions', () => {
		expect([...(index.relatedTables.get('shop.orders') ?? [])].sort()).toEqual([
			'shop.customers',
			'shop.order_items'
		])
	})
})
