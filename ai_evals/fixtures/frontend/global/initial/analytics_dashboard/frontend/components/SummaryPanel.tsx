import React, { useMemo } from 'react'
import type { Order } from '../data/seedData'
import { summarizeRevenue } from '../lib/aggregations'
import { formatCurrency, formatCurrencyPrecise, formatNumber } from '../lib/format'

interface SummaryPanelProps {
	orders: Order[]
	loading?: boolean
}

// Headline revenue panel. It re-aggregates the orders client-side via
// summarizeRevenue so the totals stay in sync with whatever filter the user
// has applied, without waiting for another backend round trip.
export const SummaryPanel: React.FC<SummaryPanelProps> = ({ orders, loading }) => {
	const summary = useMemo(() => summarizeRevenue(orders), [orders])

	const tiles = [
		{ label: 'Total Revenue', value: formatCurrency(summary.totalRevenue), emphasis: true },
		{ label: 'Net Revenue', value: formatCurrency(summary.netRevenue) },
		{ label: 'Orders', value: formatNumber(summary.totalOrders) },
		{ label: 'Avg Order Value', value: formatCurrencyPrecise(summary.averageOrderValue) },
		{ label: 'Units Sold', value: formatNumber(summary.unitsSold) },
		{ label: 'Refunded', value: formatCurrency(summary.refundedRevenue) }
	]

	return (
		<section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
			<div className="mb-4 flex items-center justify-between">
				<h2 className="text-lg font-semibold text-gray-900">Revenue Summary</h2>
				{loading ? <span className="text-xs text-gray-400">Refreshing…</span> : null}
			</div>
			<div className="grid grid-cols-2 gap-4 md:grid-cols-3">
				{tiles.map((tile) => (
					<div
						key={tile.label}
						className={`rounded-lg p-4 ${tile.emphasis ? 'bg-indigo-50' : 'bg-gray-50'}`}
					>
						<div className="text-xs font-medium uppercase tracking-wide text-gray-500">
							{tile.label}
						</div>
						<div
							className={`mt-1 font-bold ${tile.emphasis ? 'text-2xl text-indigo-700' : 'text-xl text-gray-900'}`}
						>
							{tile.value}
						</div>
					</div>
				))}
			</div>
		</section>
	)
}
