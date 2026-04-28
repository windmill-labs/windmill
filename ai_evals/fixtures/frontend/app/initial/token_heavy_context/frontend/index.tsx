import React, { useEffect, useMemo, useState } from 'react'
import { backend } from 'wmill'

type Metric = {
	id: number
	label: string
	description: string
}

const referenceMetrics: Metric[] = [
	{ id: 1, label: 'Reference metric 1', description: 'This is intentionally verbose dashboard reference text number 1 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 2, label: 'Reference metric 2', description: 'This is intentionally verbose dashboard reference text number 2 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 3, label: 'Reference metric 3', description: 'This is intentionally verbose dashboard reference text number 3 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 4, label: 'Reference metric 4', description: 'This is intentionally verbose dashboard reference text number 4 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 5, label: 'Reference metric 5', description: 'This is intentionally verbose dashboard reference text number 5 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 6, label: 'Reference metric 6', description: 'This is intentionally verbose dashboard reference text number 6 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 7, label: 'Reference metric 7', description: 'This is intentionally verbose dashboard reference text number 7 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 8, label: 'Reference metric 8', description: 'This is intentionally verbose dashboard reference text number 8 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 9, label: 'Reference metric 9', description: 'This is intentionally verbose dashboard reference text number 9 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 10, label: 'Reference metric 10', description: 'This is intentionally verbose dashboard reference text number 10 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 11, label: 'Reference metric 11', description: 'This is intentionally verbose dashboard reference text number 11 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 12, label: 'Reference metric 12', description: 'This is intentionally verbose dashboard reference text number 12 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 13, label: 'Reference metric 13', description: 'This is intentionally verbose dashboard reference text number 13 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 14, label: 'Reference metric 14', description: 'This is intentionally verbose dashboard reference text number 14 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 15, label: 'Reference metric 15', description: 'This is intentionally verbose dashboard reference text number 15 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 16, label: 'Reference metric 16', description: 'This is intentionally verbose dashboard reference text number 16 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 17, label: 'Reference metric 17', description: 'This is intentionally verbose dashboard reference text number 17 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 18, label: 'Reference metric 18', description: 'This is intentionally verbose dashboard reference text number 18 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 19, label: 'Reference metric 19', description: 'This is intentionally verbose dashboard reference text number 19 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 20, label: 'Reference metric 20', description: 'This is intentionally verbose dashboard reference text number 20 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 21, label: 'Reference metric 21', description: 'This is intentionally verbose dashboard reference text number 21 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 22, label: 'Reference metric 22', description: 'This is intentionally verbose dashboard reference text number 22 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 23, label: 'Reference metric 23', description: 'This is intentionally verbose dashboard reference text number 23 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 24, label: 'Reference metric 24', description: 'This is intentionally verbose dashboard reference text number 24 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 25, label: 'Reference metric 25', description: 'This is intentionally verbose dashboard reference text number 25 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 26, label: 'Reference metric 26', description: 'This is intentionally verbose dashboard reference text number 26 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 27, label: 'Reference metric 27', description: 'This is intentionally verbose dashboard reference text number 27 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 28, label: 'Reference metric 28', description: 'This is intentionally verbose dashboard reference text number 28 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 29, label: 'Reference metric 29', description: 'This is intentionally verbose dashboard reference text number 29 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 30, label: 'Reference metric 30', description: 'This is intentionally verbose dashboard reference text number 30 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 31, label: 'Reference metric 31', description: 'This is intentionally verbose dashboard reference text number 31 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 32, label: 'Reference metric 32', description: 'This is intentionally verbose dashboard reference text number 32 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 33, label: 'Reference metric 33', description: 'This is intentionally verbose dashboard reference text number 33 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 34, label: 'Reference metric 34', description: 'This is intentionally verbose dashboard reference text number 34 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 35, label: 'Reference metric 35', description: 'This is intentionally verbose dashboard reference text number 35 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 36, label: 'Reference metric 36', description: 'This is intentionally verbose dashboard reference text number 36 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 37, label: 'Reference metric 37', description: 'This is intentionally verbose dashboard reference text number 37 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 38, label: 'Reference metric 38', description: 'This is intentionally verbose dashboard reference text number 38 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 39, label: 'Reference metric 39', description: 'This is intentionally verbose dashboard reference text number 39 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 40, label: 'Reference metric 40', description: 'This is intentionally verbose dashboard reference text number 40 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 41, label: 'Reference metric 41', description: 'This is intentionally verbose dashboard reference text number 41 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 42, label: 'Reference metric 42', description: 'This is intentionally verbose dashboard reference text number 42 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 43, label: 'Reference metric 43', description: 'This is intentionally verbose dashboard reference text number 43 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 44, label: 'Reference metric 44', description: 'This is intentionally verbose dashboard reference text number 44 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 45, label: 'Reference metric 45', description: 'This is intentionally verbose dashboard reference text number 45 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 46, label: 'Reference metric 46', description: 'This is intentionally verbose dashboard reference text number 46 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 47, label: 'Reference metric 47', description: 'This is intentionally verbose dashboard reference text number 47 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 48, label: 'Reference metric 48', description: 'This is intentionally verbose dashboard reference text number 48 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 49, label: 'Reference metric 49', description: 'This is intentionally verbose dashboard reference text number 49 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 50, label: 'Reference metric 50', description: 'This is intentionally verbose dashboard reference text number 50 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 51, label: 'Reference metric 51', description: 'This is intentionally verbose dashboard reference text number 51 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 52, label: 'Reference metric 52', description: 'This is intentionally verbose dashboard reference text number 52 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 53, label: 'Reference metric 53', description: 'This is intentionally verbose dashboard reference text number 53 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 54, label: 'Reference metric 54', description: 'This is intentionally verbose dashboard reference text number 54 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 55, label: 'Reference metric 55', description: 'This is intentionally verbose dashboard reference text number 55 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 56, label: 'Reference metric 56', description: 'This is intentionally verbose dashboard reference text number 56 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 57, label: 'Reference metric 57', description: 'This is intentionally verbose dashboard reference text number 57 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 58, label: 'Reference metric 58', description: 'This is intentionally verbose dashboard reference text number 58 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 59, label: 'Reference metric 59', description: 'This is intentionally verbose dashboard reference text number 59 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 60, label: 'Reference metric 60', description: 'This is intentionally verbose dashboard reference text number 60 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 61, label: 'Reference metric 61', description: 'This is intentionally verbose dashboard reference text number 61 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 62, label: 'Reference metric 62', description: 'This is intentionally verbose dashboard reference text number 62 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 63, label: 'Reference metric 63', description: 'This is intentionally verbose dashboard reference text number 63 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 64, label: 'Reference metric 64', description: 'This is intentionally verbose dashboard reference text number 64 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 65, label: 'Reference metric 65', description: 'This is intentionally verbose dashboard reference text number 65 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 66, label: 'Reference metric 66', description: 'This is intentionally verbose dashboard reference text number 66 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 67, label: 'Reference metric 67', description: 'This is intentionally verbose dashboard reference text number 67 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 68, label: 'Reference metric 68', description: 'This is intentionally verbose dashboard reference text number 68 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 69, label: 'Reference metric 69', description: 'This is intentionally verbose dashboard reference text number 69 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 70, label: 'Reference metric 70', description: 'This is intentionally verbose dashboard reference text number 70 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 71, label: 'Reference metric 71', description: 'This is intentionally verbose dashboard reference text number 71 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 72, label: 'Reference metric 72', description: 'This is intentionally verbose dashboard reference text number 72 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 73, label: 'Reference metric 73', description: 'This is intentionally verbose dashboard reference text number 73 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 74, label: 'Reference metric 74', description: 'This is intentionally verbose dashboard reference text number 74 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 75, label: 'Reference metric 75', description: 'This is intentionally verbose dashboard reference text number 75 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 76, label: 'Reference metric 76', description: 'This is intentionally verbose dashboard reference text number 76 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 77, label: 'Reference metric 77', description: 'This is intentionally verbose dashboard reference text number 77 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 78, label: 'Reference metric 78', description: 'This is intentionally verbose dashboard reference text number 78 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 79, label: 'Reference metric 79', description: 'This is intentionally verbose dashboard reference text number 79 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 80, label: 'Reference metric 80', description: 'This is intentionally verbose dashboard reference text number 80 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 81, label: 'Reference metric 81', description: 'This is intentionally verbose dashboard reference text number 81 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 82, label: 'Reference metric 82', description: 'This is intentionally verbose dashboard reference text number 82 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 83, label: 'Reference metric 83', description: 'This is intentionally verbose dashboard reference text number 83 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 84, label: 'Reference metric 84', description: 'This is intentionally verbose dashboard reference text number 84 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 85, label: 'Reference metric 85', description: 'This is intentionally verbose dashboard reference text number 85 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 86, label: 'Reference metric 86', description: 'This is intentionally verbose dashboard reference text number 86 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 87, label: 'Reference metric 87', description: 'This is intentionally verbose dashboard reference text number 87 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 88, label: 'Reference metric 88', description: 'This is intentionally verbose dashboard reference text number 88 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 89, label: 'Reference metric 89', description: 'This is intentionally verbose dashboard reference text number 89 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 90, label: 'Reference metric 90', description: 'This is intentionally verbose dashboard reference text number 90 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 91, label: 'Reference metric 91', description: 'This is intentionally verbose dashboard reference text number 91 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 92, label: 'Reference metric 92', description: 'This is intentionally verbose dashboard reference text number 92 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 93, label: 'Reference metric 93', description: 'This is intentionally verbose dashboard reference text number 93 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 94, label: 'Reference metric 94', description: 'This is intentionally verbose dashboard reference text number 94 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 95, label: 'Reference metric 95', description: 'This is intentionally verbose dashboard reference text number 95 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 96, label: 'Reference metric 96', description: 'This is intentionally verbose dashboard reference text number 96 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 97, label: 'Reference metric 97', description: 'This is intentionally verbose dashboard reference text number 97 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 98, label: 'Reference metric 98', description: 'This is intentionally verbose dashboard reference text number 98 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 99, label: 'Reference metric 99', description: 'This is intentionally verbose dashboard reference text number 99 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 100, label: 'Reference metric 100', description: 'This is intentionally verbose dashboard reference text number 100 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 101, label: 'Reference metric 101', description: 'This is intentionally verbose dashboard reference text number 101 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 102, label: 'Reference metric 102', description: 'This is intentionally verbose dashboard reference text number 102 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 103, label: 'Reference metric 103', description: 'This is intentionally verbose dashboard reference text number 103 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 104, label: 'Reference metric 104', description: 'This is intentionally verbose dashboard reference text number 104 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 105, label: 'Reference metric 105', description: 'This is intentionally verbose dashboard reference text number 105 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 106, label: 'Reference metric 106', description: 'This is intentionally verbose dashboard reference text number 106 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 107, label: 'Reference metric 107', description: 'This is intentionally verbose dashboard reference text number 107 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 108, label: 'Reference metric 108', description: 'This is intentionally verbose dashboard reference text number 108 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 109, label: 'Reference metric 109', description: 'This is intentionally verbose dashboard reference text number 109 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 110, label: 'Reference metric 110', description: 'This is intentionally verbose dashboard reference text number 110 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 111, label: 'Reference metric 111', description: 'This is intentionally verbose dashboard reference text number 111 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 112, label: 'Reference metric 112', description: 'This is intentionally verbose dashboard reference text number 112 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 113, label: 'Reference metric 113', description: 'This is intentionally verbose dashboard reference text number 113 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 114, label: 'Reference metric 114', description: 'This is intentionally verbose dashboard reference text number 114 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 115, label: 'Reference metric 115', description: 'This is intentionally verbose dashboard reference text number 115 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 116, label: 'Reference metric 116', description: 'This is intentionally verbose dashboard reference text number 116 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 117, label: 'Reference metric 117', description: 'This is intentionally verbose dashboard reference text number 117 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 118, label: 'Reference metric 118', description: 'This is intentionally verbose dashboard reference text number 118 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 119, label: 'Reference metric 119', description: 'This is intentionally verbose dashboard reference text number 119 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 120, label: 'Reference metric 120', description: 'This is intentionally verbose dashboard reference text number 120 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 121, label: 'Reference metric 121', description: 'This is intentionally verbose dashboard reference text number 121 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 122, label: 'Reference metric 122', description: 'This is intentionally verbose dashboard reference text number 122 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 123, label: 'Reference metric 123', description: 'This is intentionally verbose dashboard reference text number 123 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 124, label: 'Reference metric 124', description: 'This is intentionally verbose dashboard reference text number 124 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 125, label: 'Reference metric 125', description: 'This is intentionally verbose dashboard reference text number 125 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 126, label: 'Reference metric 126', description: 'This is intentionally verbose dashboard reference text number 126 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 127, label: 'Reference metric 127', description: 'This is intentionally verbose dashboard reference text number 127 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 128, label: 'Reference metric 128', description: 'This is intentionally verbose dashboard reference text number 128 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 129, label: 'Reference metric 129', description: 'This is intentionally verbose dashboard reference text number 129 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 130, label: 'Reference metric 130', description: 'This is intentionally verbose dashboard reference text number 130 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 131, label: 'Reference metric 131', description: 'This is intentionally verbose dashboard reference text number 131 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 132, label: 'Reference metric 132', description: 'This is intentionally verbose dashboard reference text number 132 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 133, label: 'Reference metric 133', description: 'This is intentionally verbose dashboard reference text number 133 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 134, label: 'Reference metric 134', description: 'This is intentionally verbose dashboard reference text number 134 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 135, label: 'Reference metric 135', description: 'This is intentionally verbose dashboard reference text number 135 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 136, label: 'Reference metric 136', description: 'This is intentionally verbose dashboard reference text number 136 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 137, label: 'Reference metric 137', description: 'This is intentionally verbose dashboard reference text number 137 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 138, label: 'Reference metric 138', description: 'This is intentionally verbose dashboard reference text number 138 used to simulate a large selected app file with lots of inline business copy and configuration.' },
	{ id: 139, label: 'Reference metric 139', description: 'This is intentionally verbose dashboard reference text number 139 used to simulate a large selected app file with lots of inline business copy and configuration.' },
]

export default function AnalyticsConsole() {
	const [summary, setSummary] = useState<string>('Loading analytics summary...')
	const [filter, setFilter] = useState('')

	useEffect(() => {
		backend.loadAnalytics({ range: '30d' }).then((result) => {
			setSummary(result.summary)
		})
	}, [])

	const visibleMetrics = useMemo(() => {
		return referenceMetrics.filter((metric) =>
			metric.label.toLowerCase().includes(filter.toLowerCase()) ||
			metric.description.toLowerCase().includes(filter.toLowerCase())
		)
	}, [filter])

	return (
		<main className="min-h-screen bg-slate-950 text-white p-8">
			<section className="max-w-5xl mx-auto space-y-6">
				<div>
					<p className="uppercase tracking-wide text-xs text-cyan-300">Windmill Analytics</p>
					<h1 className="text-4xl font-bold">Analytics Console</h1>
					<p className="text-slate-300 mt-2">{summary}</p>
				</div>
				<input
					className="w-full rounded bg-slate-800 border border-slate-700 p-3"
					placeholder="Filter reference metrics"
					value={filter}
					onChange={(event) => setFilter(event.target.value)}
				/>
				<div className="grid grid-cols-1 md:grid-cols-2 gap-4">
					{visibleMetrics.slice(0, 20).map((metric) => (
						<article key={metric.id} className="rounded border border-slate-800 p-4 bg-slate-900">
							<h2 className="font-semibold">{metric.label}</h2>
							<p className="text-sm text-slate-400">{metric.description}</p>
						</article>
					))}
				</div>
			</section>
		</main>
	)
}
