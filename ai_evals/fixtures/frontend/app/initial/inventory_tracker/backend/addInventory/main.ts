import * as wmill from 'windmill-client'

interface InventoryItem {
	id: number
	name: string
	sku: string
	quantity: number
	price: number
	created_at: string
}

export async function main({
	name,
	sku,
	quantity,
	price
}: {
	name: string
	sku: string
	quantity: number
	price: number
}): Promise<InventoryItem> {
	const sql = wmill.datatable()
	return await sql`
		INSERT INTO public.inventory_items (name, sku, quantity, price)
		VALUES (${name}, ${sku}, ${quantity}, ${price})
		RETURNING id, name, sku, quantity, price, created_at
	`.fetchOne()
}
