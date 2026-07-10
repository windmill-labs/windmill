import React, { useState } from 'react'
import { requestExport } from '../lib/api'
import type { DateRange } from '../lib/api'

interface ExportButtonProps {
	range: DateRange
	region: string
}

export const ExportButton: React.FC<ExportButtonProps> = ({ range, region }) => {
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const handleExport = async (format: 'csv' | 'json') => {
		setBusy(true)
		setError(null)
		try {
			const result = await requestExport(range, region, format)
			const anchor = document.createElement('a')
			anchor.href = result.url
			anchor.download = `revenue-report.${format}`
			anchor.click()
		} catch (err) {
			setError(err instanceof Error ? err.message : 'Export failed')
		} finally {
			setBusy(false)
		}
	}

	return (
		<div className="flex items-center gap-2">
			<button
				type="button"
				disabled={busy}
				onClick={() => handleExport('csv')}
				className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
			>
				{busy ? 'Exporting…' : 'Export CSV'}
			</button>
			<button
				type="button"
				disabled={busy}
				onClick={() => handleExport('json')}
				className="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50"
			>
				Export JSON
			</button>
			{error ? <span className="text-xs text-rose-600">{error}</span> : null}
		</div>
	)
}
