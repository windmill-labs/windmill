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
	product
}: {
	cart: CartItem[]
	product: Product
}): Promise<CartItem[]> {
	const existingIndex = cart.findIndex((item) => item.product.id === product.id)

	if (existingIndex >= 0) {
		// Increment quantity if already in cart
		const updatedCart = [...cart]
		updatedCart[existingIndex] = {
			...updatedCart[existingIndex],
			quantity: updatedCart[existingIndex].quantity + 1
		}
		return updatedCart
	}

	// Add new item to cart
	return [...cart, { product, quantity: 1 }]
}
