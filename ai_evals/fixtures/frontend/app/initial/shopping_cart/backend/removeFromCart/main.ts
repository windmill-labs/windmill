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

export async function main({
	cart,
	productId
}: {
	cart: CartItem[]
	productId: string
}): Promise<CartItem[]> {
	return cart.filter((item) => item.product.id !== productId)
}
