import React, { useState, useEffect } from 'react'
import { backend } from 'wmill'
import { ProductList } from './components/ProductList'
import { Cart } from './components/Cart'

export interface Product {
	id: string
	name: string
	price: number
	image: string
}

export interface CartItem {
	product: Product
	quantity: number
}

const App = () => {
	const [products, setProducts] = useState<Product[]>([])
	const [cartItems, setCartItems] = useState<CartItem[]>([])
	const [total, setTotal] = useState(0)
	const [loading, setLoading] = useState(true)

	useEffect(() => {
		loadProducts()
	}, [])

	useEffect(() => {
		updateTotal()
	}, [cartItems])

	const loadProducts = async () => {
		setLoading(true)
		const data = await backend.getProducts()
		setProducts(data)
		setLoading(false)
	}

	const updateTotal = async () => {
		if (cartItems.length === 0) {
			setTotal(0)
			return
		}
		const newTotal = await backend.calculateTotal({ items: cartItems })
		setTotal(newTotal)
	}

	const handleAddToCart = async (product: Product) => {
		const updatedCart = await backend.addToCart({
			cart: cartItems,
			product
		})
		setCartItems(updatedCart)
	}

	const handleRemoveFromCart = async (productId: string) => {
		const updatedCart = await backend.removeFromCart({
			cart: cartItems,
			productId
		})
		setCartItems(updatedCart)
	}

	if (loading) {
		return <div className="p-8 text-center">Loading products...</div>
	}

	return (
		<div className="flex h-screen">
			<div className="flex-1 p-6 overflow-auto">
				<h1 className="text-2xl font-bold mb-6">Shop</h1>
				<ProductList products={products} onAddToCart={handleAddToCart} />
			</div>
			<div className="w-80 border-l bg-gray-50 p-6">
				<Cart items={cartItems} total={total} onRemoveItem={handleRemoveFromCart} />
			</div>
		</div>
	)
}

export default App
