import React from 'react'
import { ProductCard } from './ProductCard'
import type { Product } from '../index'

interface ProductListProps {
	products: Product[]
	onAddToCart: (product: Product) => void
}

export const ProductList: React.FC<ProductListProps> = ({ products, onAddToCart }) => {
	return (
		<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
			{products.map((product) => (
				<ProductCard key={product.id} product={product} onAddToCart={onAddToCart} />
			))}
		</div>
	)
}
