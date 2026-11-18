import React, { useMemo } from 'react'
import type { Order } from '../data/seedData'
import { dailyRevenue } from '../lib/aggregations'
import { formatCompact, formatDateShort } from '../lib/format'
import { EmptyState } from './EmptyState'

interface RevenueChartProps {
	orders: Order[]
}

// Lightweight inline bar chart for daily revenue. Avoids a charting dependency
// by sizing flexed columns relative to the busiest day in the window.
export const RevenueChart: React.FC<RevenueChartProps> = ({ orders }) => {
	const points = useMemo(() => dailyRevenue(orders), [orders])
	const max = useMemo(() => points.reduce((acc, point) => Math.max(acc, point.revenue), 0), [points])

	if (points.length === 0) {
		return (
			<EmptyState
				title="No revenue in range"
				description="Adjust the date range or filters to see daily revenue."
				icon="📉"
			/>
		)
	}

	return (
		<section className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm">
			<h2 className="mb-4 text-lg font-semibold text-gray-900">Daily Revenue</h2>
			<div className="flex h-48 items-end gap-1">
				{points.map((point) => {
					const heightPct = max === 0 ? 0 : Math.round((point.revenue / max) * 100)
					return (
						<div key={point.date} className="flex flex-1 flex-col items-center justify-end">
							<div
								className="w-full rounded-t bg-indigo-400"
								style={{ height: `${Math.max(heightPct, 2)}%` }}
								title={`${point.date}: ${formatCompact(point.revenue)}`}
							/>
							<span className="mt-1 truncate text-[9px] text-gray-400">
								{formatDateShort(point.date)}
							</span>
						</div>
					)
				})}
			</div>
		</section>
	)
}
