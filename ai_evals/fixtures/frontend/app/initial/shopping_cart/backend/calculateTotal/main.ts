interface Product {
	id: string
	name: string
	price: number
	image: string
}

interface CartItem {
	product: Product
	quantity: number
}

export async function main({ items }: { items: CartItem[] }): Promise<number> {
	return items.reduce((total, item) => {
		return total + item.product.price * item.quantity
	}, 0)
}
