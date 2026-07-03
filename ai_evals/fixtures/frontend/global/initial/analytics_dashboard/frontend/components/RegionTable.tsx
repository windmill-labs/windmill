import React, { useMemo } from 'react'
import type { Order } from '../data/seedData'
import { breakdownByRegion } from '../lib/aggregations'
import { formatCurrency, formatNumber, formatPercent } from '../lib/format'
import { EmptyState } from './EmptyState'

interface RegionTableProps {
	orders: Order[]
}

export const RegionTable: React.FC<RegionTableProps> = ({ orders }) => {
	const rows = useMemo(() => breakdownByRegion(orders), [orders])
	const total = useMemo(() => rows.reduce((acc, row) => acc + row.revenue, 0), [rows])

	if (rows.length === 0) {
		return (
			<EmptyState
				title="No regional revenue"
				description="No revenue-bearing orders fall in the current selection."
				icon="🌍"
			/>
		)
	}

	return (
		<section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
			<h2 className="mb-4 text-lg font-semibold text-gray-900">Revenue by Region</h2>
			<table className="min-w-full text-sm">
				<thead className="text-left text-xs uppercase tracking-wide text-gray-500">
					<tr>
						<th className="py-2">Region</th>
						<th className="py-2 text-right">Orders</th>
						<th className="py-2 text-right">Revenue</th>
						<th className="py-2 text-right">Share</th>
					</tr>
				</thead>
				<tbody className="divide-y divide-gray-100">
					{rows.map((row) => (
						<tr key={row.region}>
							<td className="py-2 font-medium text-gray-900">{row.region}</td>
							<td className="py-2 text-right text-gray-600">{formatNumber(row.orders)}</td>
							<td className="py-2 text-right text-gray-900">{formatCurrency(row.revenue)}</td>
							<td className="py-2 text-right text-gray-500">
								{formatPercent(total === 0 ? 0 : row.revenue / total)}
							</td>
						</tr>
					))}
				</tbody>
			</table>
		</section>
	)
}
