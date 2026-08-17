import * as wmill from 'windmill-client'

interface InventoryItem {
	id: number
	name: string
	sku: string
	quantity: number
	price: number
	created_at: string
}

export async function main(): Promise<InventoryItem[]> {
	const sql = wmill.datatable()
	return await sql`
		SELECT id, name, sku, quantity, price, created_at
		FROM public.inventory_items
		ORDER BY created_at DESC
	`.fetch()
}
