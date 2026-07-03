import React from 'react'
import type { DateRange } from '../lib/api'
import { rangeForPreset } from '../lib/api'
import { formatDateShort } from '../lib/format'

interface DateRangePickerProps {
	preset: string
	range: DateRange
	onPresetChange: (preset: string, range: DateRange) => void
}

const PRESETS: { id: string; label: string }[] = [
	{ id: '7d', label: 'Last 7 days' },
	{ id: '14d', label: 'Last 14 days' },
	{ id: '30d', label: 'Last 30 days' },
	{ id: 'qtd', label: 'Quarter to date' }
]

export const DateRangePicker: React.FC<DateRangePickerProps> = ({
	preset,
	range,
	onPresetChange
}) => {
	return (
		<div className="flex items-center gap-2">
			<select
				className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-700"
				value={preset}
				onChange={(event) => {
					const next = event.target.value
					onPresetChange(next, rangeForPreset(next))
				}}
			>
				{PRESETS.map((item) => (
					<option key={item.id} value={item.id}>
						{item.label}
					</option>
				))}
			</select>
			<span className="text-xs text-gray-400">
				{formatDateShort(range.from)} – {formatDateShort(range.to)}
			</span>
		</div>
	)
}
