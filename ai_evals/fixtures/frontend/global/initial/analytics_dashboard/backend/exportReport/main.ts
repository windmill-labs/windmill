// Builds a downloadable report for the current dashboard view. Returns a data
// URL the browser can open directly so the export works without object storage.
export async function main({
	from,
	to,
	region,
	format
}: {
	from: string
	to: string
	region: string
	format: 'csv' | 'json'
}): Promise<{ url: string; rows: number; filename: string }> {
	const summary = {
		from,
		to,
		region: region || 'all',
		generatedAt: new Date().toISOString(),
		rows: [
			{ region: 'North America', revenue: 211_400, orders: 168 },
			{ region: 'EMEA', revenue: 142_900, orders: 121 },
			{ region: 'APAC', revenue: 86_500, orders: 78 },
			{ region: 'LATAM', revenue: 41_500, orders: 45 }
		]
	}

	const scoped =
		region && region !== 'all'
			? summary.rows.filter((row) => row.region === region)
			: summary.rows

	let body: string
	let mime: string
	if (format === 'csv') {
		const header = 'region,revenue,orders'
		const lines = scoped.map((row) => `${row.region},${row.revenue},${row.orders}`)
		body = [header, ...lines].join('\n')
		mime = 'text/csv'
	} else {
		body = JSON.stringify({ ...summary, rows: scoped }, null, 2)
		mime = 'application/json'
	}

	const encoded = Buffer.from(body, 'utf-8').toString('base64')
	const filename = `revenue-report-${from}_${to}.${format}`
	return {
		url: `data:${mime};base64,${encoded}`,
		rows: scoped.length,
		filename
	}
}
