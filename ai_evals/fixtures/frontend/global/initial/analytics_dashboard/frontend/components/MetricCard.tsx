import React from 'react'
import type { MetricCardData } from '../data/seedData'
import { formatCurrency, formatNumber, formatPercent, formatSignedPercent } from '../lib/format'

interface MetricCardProps {
	metric: MetricCardData
	loading?: boolean
}

function renderValue(metric: MetricCardData): string {
	switch (metric.unit) {
		case 'currency':
			return formatCurrency(metric.value)
		case 'percent':
			return formatPercent(metric.value)
		case 'count':
		default:
			return formatNumber(metric.value)
	}
}

export const MetricCard: React.FC<MetricCardProps> = ({ metric, loading }) => {
	const positive = metric.delta >= 0
	return (
		<div className="rounded-xl border border-gray-200 bg-white p-5 shadow-sm">
			<div className="flex items-center justify-between">
				<span className="text-sm font-medium text-gray-500">{metric.label}</span>
				<span
					className={`text-xs font-semibold ${positive ? 'text-emerald-600' : 'text-rose-600'}`}
				>
					{formatSignedPercent(metric.delta)}
				</span>
			</div>
			<div className="mt-2 text-2xl font-bold text-gray-900">
				{loading ? <span className="text-gray-300">…</span> : renderValue(metric)}
			</div>
			<p className="mt-1 text-xs text-gray-400">{metric.hint}</p>
		</div>
	)
}
