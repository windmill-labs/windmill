import React from 'react'
import type { MetricCardData } from '../data/seedData'
import { MetricCard } from './MetricCard'

interface MetricGridProps {
	metrics: MetricCardData[]
	loading?: boolean
}

export const MetricGrid: React.FC<MetricGridProps> = ({ metrics, loading }) => {
	return (
		<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{metrics.map((metric) => (
				<MetricCard key={metric.id} metric={metric} loading={loading} />
			))}
		</div>
	)
}
