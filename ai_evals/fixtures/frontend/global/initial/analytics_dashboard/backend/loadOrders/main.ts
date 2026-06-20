type OrderStatus = 'paid' | 'shipped' | 'delivered' | 'pending' | 'refunded' | 'cancelled'

interface Order {
	id: string
	placedAt: string
	customer: string
	product: string
	sku: string
	region: string
	channel: string
	rep: string
	quantity: number
	unitPrice: number
	status: OrderStatus
}

// Mocked order book. In a real deployment this would query the orders table;
// here it returns a representative slice so the table renders in preview.
const orders: Order[] = [
	{ id: 'ORD-10001', placedAt: '2024-05-02T09:14:00Z', customer: 'Contoso Ltd', product: 'Aurora Analytics Suite', sku: 'ANL-100', region: 'North America', channel: 'direct', rep: 'Dana Wills', quantity: 3, unitPrice: 1195, status: 'delivered' },
	{ id: 'ORD-10002', placedAt: '2024-05-03T11:42:00Z', customer: 'Fabrikam Inc', product: 'Borealis CRM', sku: 'CRM-210', region: 'EMEA', channel: 'partner', rep: 'Lena Fischer', quantity: 5, unitPrice: 880, status: 'shipped' },
	{ id: 'ORD-10003', placedAt: '2024-05-05T15:03:00Z', customer: 'Tailspin Toys', product: 'Cascade Data Pipeline', sku: 'PIPE-330', region: 'APAC', channel: 'self-serve', rep: 'Sora Tanaka', quantity: 2, unitPrice: 640, status: 'paid' },
	{ id: 'ORD-10004', placedAt: '2024-05-07T08:21:00Z', customer: 'Proseware Inc', product: 'Delta Insights', sku: 'INS-440', region: 'LATAM', channel: 'marketplace', rep: 'Diego Marin', quantity: 7, unitPrice: 315, status: 'delivered' },
	{ id: 'ORD-10005', placedAt: '2024-05-09T13:58:00Z', customer: 'Litware Inc', product: 'Echo Monitoring', sku: 'MON-550', region: 'North America', channel: 'direct', rep: 'Owen Pratt', quantity: 4, unitPrice: 150, status: 'refunded' },
	{ id: 'ORD-10006', placedAt: '2024-05-12T10:30:00Z', customer: 'Fourth Coffee', product: 'Helix Identity', sku: 'IDN-880', region: 'EMEA', channel: 'partner', rep: 'Aisha Khan', quantity: 6, unitPrice: 220, status: 'shipped' },
	{ id: 'ORD-10007', placedAt: '2024-05-15T17:11:00Z', customer: 'Coho Vineyard', product: 'Kelvin Forecasting', sku: 'FCT-202', region: 'APAC', channel: 'direct', rep: 'Priya Nair', quantity: 1, unitPrice: 980, status: 'pending' },
	{ id: 'ORD-10008', placedAt: '2024-05-18T12:05:00Z', customer: 'Alpine Ski House', product: 'Nimbus Compute', sku: 'CMP-505', region: 'North America', channel: 'self-serve', rep: 'Hugo Bernard', quantity: 8, unitPrice: 1100, status: 'delivered' },
	{ id: 'ORD-10009', placedAt: '2024-05-22T14:47:00Z', customer: 'Trey Research', product: 'Onyx Security', sku: 'SEC-606', region: 'EMEA', channel: 'direct', rep: 'Sven Olsen', quantity: 2, unitPrice: 860, status: 'cancelled' },
	{ id: 'ORD-10010', placedAt: '2024-05-26T16:39:00Z', customer: 'Blue Yonder Airlines', product: 'Polaris Reporting', sku: 'RPT-707', region: 'LATAM', channel: 'partner', rep: 'Mateo Russo', quantity: 9, unitPrice: 290, status: 'paid' }
]

export async function main({
	from,
	to,
	region,
	status
}: {
	from: string
	to: string
	region: string
	status: string
}): Promise<{ orders: Order[]; total: number }> {
	let filtered = orders.filter((order) => {
		const day = order.placedAt.slice(0, 10)
		return day >= from && day <= to
	})
	if (region && region !== 'all') {
		filtered = filtered.filter((order) => order.region === region)
	}
	if (status && status !== 'all') {
		filtered = filtered.filter((order) => order.status === status)
	}
	return { orders: filtered, total: filtered.length }
}
