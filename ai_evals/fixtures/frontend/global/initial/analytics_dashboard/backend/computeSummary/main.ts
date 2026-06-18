type OrderStatus = 'paid' | 'shipped' | 'delivered' | 'pending' | 'refunded' | 'cancelled'

interface Order {
	id: string
	region: string
	quantity: number
	unitPrice: number
	status: OrderStatus
	placedAt: string
}

// Server-side revenue rollup. Mirrors the client aggregation but is computed
// from the authoritative mocked order book so it can be used to cross-check
// the dashboard and to back the export.
const orders: Order[] = [
	{ id: 'ORD-10001', region: 'North America', quantity: 3, unitPrice: 1195, status: 'delivered', placedAt: '2024-05-02' },
	{ id: 'ORD-10002', region: 'EMEA', quantity: 5, unitPrice: 880, status: 'shipped', placedAt: '2024-05-03' },
	{ id: 'ORD-10003', region: 'APAC', quantity: 2, unitPrice: 640, status: 'paid', placedAt: '2024-05-05' },
	{ id: 'ORD-10004', region: 'LATAM', quantity: 7, unitPrice: 315, status: 'delivered', placedAt: '2024-05-07' },
	{ id: 'ORD-10005', region: 'North America', quantity: 4, unitPrice: 150, status: 'refunded', placedAt: '2024-05-09' },
	{ id: 'ORD-10006', region: 'EMEA', quantity: 6, unitPrice: 220, status: 'shipped', placedAt: '2024-05-12' },
	{ id: 'ORD-10007', region: 'APAC', quantity: 1, unitPrice: 980, status: 'pending', placedAt: '2024-05-15' },
	{ id: 'ORD-10008', region: 'North America', quantity: 8, unitPrice: 1100, status: 'delivered', placedAt: '2024-05-18' },
	{ id: 'ORD-10009', region: 'EMEA', quantity: 2, unitPrice: 860, status: 'cancelled', placedAt: '2024-05-22' },
	{ id: 'ORD-10010', region: 'LATAM', quantity: 9, unitPrice: 290, status: 'paid', placedAt: '2024-05-26' }
]

const REVENUE_STATUSES: OrderStatus[] = ['paid', 'shipped', 'delivered']

export async function main({
	from,
	to,
	region
}: {
	from: string
	to: string
	region: string
}): Promise<{
	totalRevenue: number
	netRevenue: number
	totalOrders: number
	averageOrderValue: number
	unitsSold: number
	refundedRevenue: number
	currency: string
}> {
	let scoped = orders.filter((order) => order.placedAt >= from && order.placedAt <= to)
	if (region && region !== 'all') {
		scoped = scoped.filter((order) => order.region === region)
	}

	const booked = scoped.filter((order) => REVENUE_STATUSES.includes(order.status))
	const totalRevenue = booked.reduce((acc, order) => acc + order.unitPrice * order.quantity, 0)
	const unitsSold = booked.reduce((acc, order) => acc + order.quantity, 0)
	const refundedRevenue = scoped
		.filter((order) => order.status === 'refunded')
		.reduce((acc, order) => acc + order.unitPrice * order.quantity, 0)

	return {
		totalRevenue,
		netRevenue: totalRevenue - refundedRevenue,
		totalOrders: booked.length,
		averageOrderValue: booked.length === 0 ? 0 : Math.round(totalRevenue / booked.length),
		unitsSold,
		refundedRevenue,
		currency: 'USD'
	}
}
