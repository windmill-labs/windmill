import React from 'react'
import type { Product } from '../index'

interface ProductCardProps {
	product: Product
	onAddToCart: (product: Product) => void
}

export const ProductCard: React.FC<ProductCardProps> = ({ product, onAddToCart }) => {
	return (
		<div className="border rounded-lg p-4 bg-white shadow-sm">
			<img
				src={product.image}
				alt={product.name}
				className="w-full h-48 object-cover rounded mb-4"
			/>
			<h3 className="font-semibold text-lg">{product.name}</h3>
			<p className="text-gray-600 mb-4">${product.price.toFixed(2)}</p>
			<button
				onClick={() => onAddToCart(product)}
				className="w-full py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition"
			>
				Add to Cart
			</button>
		</div>
	)
}
