interface MetricCardData {
	id: string
	label: string
	value: number
	unit: 'currency' | 'count' | 'percent'
	delta: number
	hint: string
}

// Returns the headline metric cards for the selected range and region. Values
// are mocked but internally consistent (revenue / orders ≈ avg order value).
const baseByRegion: Record<string, { revenue: number; orders: number; units: number; refunds: number }> = {
	all: { revenue: 482_300, orders: 412, units: 1840, refunds: 11_900 },
	'North America': { revenue: 211_400, orders: 168, units: 770, refunds: 4_200 },
	EMEA: { revenue: 142_900, orders: 121, units: 560, refunds: 3_500 },
	APAC: { revenue: 86_500, orders: 78, units: 340, refunds: 2_600 },
	LATAM: { revenue: 41_500, orders: 45, units: 170, refunds: 1_600 }
}

export async function main({
	from,
	to,
	region
}: {
	from: string
	to: string
	region: string
}): Promise<{ cards: MetricCardData[]; generatedAt: string }> {
	const base = baseByRegion[region] ?? baseByRegion.all
	const aov = base.orders === 0 ? 0 : Math.round(base.revenue / base.orders)
	const cards: MetricCardData[] = [
		{ id: 'revenue', label: 'Total Revenue', value: base.revenue, unit: 'currency', delta: 0.082, hint: `Booked revenue ${from} – ${to}` },
		{ id: 'orders', label: 'Orders', value: base.orders, unit: 'count', delta: 0.041, hint: 'Revenue-bearing orders in range' },
		{ id: 'aov', label: 'Avg Order Value', value: aov, unit: 'currency', delta: -0.013, hint: 'Total revenue / order count' },
		{ id: 'units', label: 'Units Sold', value: base.units, unit: 'count', delta: 0.067, hint: 'Total units in range' },
		{ id: 'refunds', label: 'Refunded', value: base.refunds, unit: 'currency', delta: -0.021, hint: 'Revenue lost to refunds' },
		{ id: 'conversion', label: 'Conversion', value: 0.187, unit: 'percent', delta: 0.009, hint: 'Sessions that became orders' }
	]
	return { cards, generatedAt: new Date().toISOString() }
}
