import React, { useMemo, useState } from 'react'
import type { Order } from '../data/seedData'
import { StatusBadge } from './StatusBadge'
import { EmptyState } from './EmptyState'
import { formatCurrencyPrecise, formatDate, formatNumber, truncate } from '../lib/format'

interface OrdersTableProps {
	orders: Order[]
	loading?: boolean
}

type SortKey = 'placedAt' | 'customer' | 'lineTotal' | 'quantity'
type SortDir = 'asc' | 'desc'

// The per-row line total a customer was charged: unit price times quantity.
function lineTotal(order: Order): number {
	return order.quantity * order.unitPrice
}

export const OrdersTable: React.FC<OrdersTableProps> = ({ orders, loading }) => {
	const [sortKey, setSortKey] = useState<SortKey>('placedAt')
	const [sortDir, setSortDir] = useState<SortDir>('desc')

	const sorted = useMemo(() => {
		const copy = [...orders]
		copy.sort((a, b) => {
			let comparison = 0
			switch (sortKey) {
				case 'customer':
					comparison = a.customer.localeCompare(b.customer)
					break
				case 'lineTotal':
					comparison = lineTotal(a) - lineTotal(b)
					break
				case 'quantity':
					comparison = a.quantity - b.quantity
					break
				case 'placedAt':
				default:
					comparison = a.placedAt.localeCompare(b.placedAt)
					break
			}
			return sortDir === 'asc' ? comparison : -comparison
		})
		return copy
	}, [orders, sortKey, sortDir])

	const toggleSort = (key: SortKey) => {
		if (key === sortKey) {
			setSortDir((dir) => (dir === 'asc' ? 'desc' : 'asc'))
		} else {
			setSortKey(key)
			setSortDir('desc')
		}
	}

	if (!loading && orders.length === 0) {
		return (
			<EmptyState
				title="No orders match these filters"
				description="Try widening the date range or clearing the status filter."
				icon="🗂️"
			/>
		)
	}

	const arrow = (key: SortKey) => (key === sortKey ? (sortDir === 'asc' ? '▲' : '▼') : '')

	return (
		<div className="overflow-hidden rounded-xl border border-gray-200 bg-white shadow-sm">
			<table className="min-w-full divide-y divide-gray-200 text-sm">
				<thead className="bg-gray-50 text-left text-xs uppercase tracking-wide text-gray-500">
					<tr>
						<th className="cursor-pointer px-4 py-3" onClick={() => toggleSort('placedAt')}>
							Date {arrow('placedAt')}
						</th>
						<th className="cursor-pointer px-4 py-3" onClick={() => toggleSort('customer')}>
							Customer {arrow('customer')}
						</th>
						<th className="px-4 py-3">Product</th>
						<th className="px-4 py-3">Region</th>
						<th className="cursor-pointer px-4 py-3 text-right" onClick={() => toggleSort('quantity')}>
							Qty {arrow('quantity')}
						</th>
						<th className="px-4 py-3 text-right">Unit Price</th>
						<th className="cursor-pointer px-4 py-3 text-right" onClick={() => toggleSort('lineTotal')}>
							Line Total {arrow('lineTotal')}
						</th>
						<th className="px-4 py-3">Status</th>
					</tr>
				</thead>
				<tbody className="divide-y divide-gray-100">
					{sorted.map((order) => (
						<tr key={order.id} className="hover:bg-gray-50">
							<td className="px-4 py-3 text-gray-500">{formatDate(order.placedAt)}</td>
							<td className="px-4 py-3 font-medium text-gray-900">
								{truncate(order.customer, 24)}
							</td>
							<td className="px-4 py-3 text-gray-600">{order.product}</td>
							<td className="px-4 py-3 text-gray-600">{order.region}</td>
							<td className="px-4 py-3 text-right text-gray-600">{formatNumber(order.quantity)}</td>
							<td className="px-4 py-3 text-right text-gray-600">
								{formatCurrencyPrecise(order.unitPrice)}
							</td>
							<td className="px-4 py-3 text-right font-semibold text-gray-900">
								{formatCurrencyPrecise(lineTotal(order))}
							</td>
							<td className="px-4 py-3">
								<StatusBadge status={order.status} />
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</div>
	)
}
