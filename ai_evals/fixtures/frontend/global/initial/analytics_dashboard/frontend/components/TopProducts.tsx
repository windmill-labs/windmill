import React, { useMemo } from 'react'
import type { Order } from '../data/seedData'
import { topProducts } from '../lib/aggregations'
import { formatCurrency } from '../lib/format'
import { EmptyState } from './EmptyState'

interface TopProductsProps {
	orders: Order[]
	limit?: number
}

export const TopProducts: React.FC<TopProductsProps> = ({ orders, limit = 5 }) => {
	const products = useMemo(() => topProducts(orders, limit), [orders, limit])
	const max = useMemo(
		() => products.reduce((acc, item) => Math.max(acc, item.revenue), 0),
		[products]
	)

	if (products.length === 0) {
		return (
			<EmptyState
				title="No product revenue"
				description="No revenue-bearing orders to rank by product."
				icon="📦"
			/>
		)
	}

	return (
		<section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
			<h2 className="mb-4 text-lg font-semibold text-gray-900">Top Products</h2>
			<ul className="space-y-3">
				{products.map((item, index) => {
					const widthPct = max === 0 ? 0 : Math.round((item.revenue / max) * 100)
					return (
						<li key={item.product}>
							<div className="flex items-center justify-between text-sm">
								<span className="font-medium text-gray-800">
									{index + 1}. {item.product}
								</span>
								<span className="text-gray-600">{formatCurrency(item.revenue)}</span>
							</div>
							<div className="mt-1 h-2 w-full overflow-hidden rounded-full bg-gray-100">
								<div
									className="h-full rounded-full bg-emerald-400"
									style={{ width: `${Math.max(widthPct, 2)}%` }}
								/>
							</div>
						</li>
					)
				})}
			</ul>
		</section>
	)
}
