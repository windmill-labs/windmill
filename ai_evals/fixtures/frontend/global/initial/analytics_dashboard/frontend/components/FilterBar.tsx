import React from 'react'
import type { DateRange } from '../lib/api'
import type { OrderStatus } from '../data/seedData'
import { REGIONS, ORDER_STATUSES, STATUS_LABELS } from '../data/seedData'
import { DateRangePicker } from './DateRangePicker'
import { ExportButton } from './ExportButton'

interface FilterBarProps {
	region: string
	status: string
	preset: string
	range: DateRange
	onRegionChange: (region: string) => void
	onStatusChange: (status: string) => void
	onPresetChange: (preset: string, range: DateRange) => void
}

export const FilterBar: React.FC<FilterBarProps> = ({
	region,
	status,
	preset,
	range,
	onRegionChange,
	onStatusChange,
	onPresetChange
}) => {
	return (
		<div className="flex flex-wrap items-center justify-between gap-3 border-b border-gray-200 bg-white px-6 py-4">
			<div className="flex flex-wrap items-center gap-3">
				<DateRangePicker preset={preset} range={range} onPresetChange={onPresetChange} />
				<select
					className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-700"
					value={region}
					onChange={(event) => onRegionChange(event.target.value)}
				>
					<option value="all">All regions</option>
					{REGIONS.map((item) => (
						<option key={item} value={item}>
							{item}
						</option>
					))}
				</select>
				<select
					className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-700"
					value={status}
					onChange={(event) => onStatusChange(event.target.value)}
				>
					<option value="all">All statuses</option>
					{ORDER_STATUSES.map((item) => (
						<option key={item} value={item}>
							{STATUS_LABELS[item as OrderStatus]}
						</option>
					))}
				</select>
			</div>
			<ExportButton range={range} region={region} />
		</div>
	)
}
