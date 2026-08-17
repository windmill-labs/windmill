import React from 'react'
import type { CartItem } from '../index'

interface CartProps {
	items: CartItem[]
	total: number
	onRemoveItem: (productId: string) => void
}

export const Cart: React.FC<CartProps> = ({ items, total, onRemoveItem }) => {
	return (
		<div className="flex flex-col h-full">
			<h2 className="text-xl font-bold mb-4">Cart</h2>

			{items.length === 0 ? (
				<p className="text-gray-500 text-center py-8">Your cart is empty</p>
			) : (
				<>
					<div className="flex-1 overflow-auto">
						{items.map((item) => (
							<div
								key={item.product.id}
								className="flex items-center justify-between py-3 border-b"
							>
								<div className="flex-1">
									<p className="font-medium">{item.product.name}</p>
									<p className="text-sm text-gray-500">
										${item.product.price.toFixed(2)} x {item.quantity}
									</p>
								</div>
								<button
									onClick={() => onRemoveItem(item.product.id)}
									className="text-red-500 hover:text-red-700 px-2"
								>
									Remove
								</button>
							</div>
						))}
					</div>

					<div className="border-t pt-4 mt-4">
						<div className="flex justify-between text-lg font-bold">
							<span>Total:</span>
							<span>${total.toFixed(2)}</span>
						</div>
					</div>
				</>
			)}
		</div>
	)
}
