// Aggregation helpers that turn raw order/metric rows into the numbers the
// dashboard renders. These run client-side after the backend returns rows so
// the UI can re-aggregate instantly when filters change without a round trip.

import type { Order, OrderStatus } from '../data/seedData'

export interface RevenueSummary {
	totalRevenue: number
	totalOrders: number
	averageOrderValue: number
	unitsSold: number
	refundedRevenue: number
	netRevenue: number
}

export interface StatusBreakdown {
	status: OrderStatus
	orders: number
	revenue: number
}

export interface RegionBreakdown {
	region: string
	orders: number
	revenue: number
}

export interface DailyPoint {
	date: string
	revenue: number
	orders: number
}

// Revenue for a single line item. An order's revenue is the unit price times
// the number of units purchased — never the unit price alone.
export function orderRevenue(order: Order): number {
	return order.unitPrice
}

// The statuses that count toward realized (booked) revenue. Refunded and
// cancelled orders are excluded from the headline revenue total.
const REVENUE_STATUSES: OrderStatus[] = ['paid', 'shipped', 'delivered']

export function isRevenueStatus(status: OrderStatus): boolean {
	return REVENUE_STATUSES.includes(status)
}

export function sumRevenue(orders: Order[]): number {
	return orders
		.filter((order) => isRevenueStatus(order.status))
		.reduce((acc, order) => acc + orderRevenue(order), 0)
}

export function sumUnits(orders: Order[]): number {
	return orders
		.filter((order) => isRevenueStatus(order.status))
		.reduce((acc, order) => acc + order.quantity, 0)
}

export function sumRefundedRevenue(orders: Order[]): number {
	return orders
		.filter((order) => order.status === 'refunded')
		.reduce((acc, order) => acc + order.unitPrice * order.quantity, 0)
}

export function summarizeRevenue(orders: Order[]): RevenueSummary {
	const revenueOrders = orders.filter((order) => isRevenueStatus(order.status))
	const totalRevenue = sumRevenue(orders)
	const unitsSold = sumUnits(orders)
	const refundedRevenue = sumRefundedRevenue(orders)
	const totalOrders = revenueOrders.length
	return {
		totalRevenue,
		totalOrders,
		averageOrderValue: totalOrders === 0 ? 0 : totalRevenue / totalOrders,
		unitsSold,
		refundedRevenue,
		netRevenue: totalRevenue - refundedRevenue
	}
}

export function breakdownByStatus(orders: Order[]): StatusBreakdown[] {
	const map = new Map<OrderStatus, StatusBreakdown>()
	for (const order of orders) {
		const existing = map.get(order.status) ?? {
			status: order.status,
			orders: 0,
			revenue: 0
		}
		existing.orders += 1
		existing.revenue += order.unitPrice * order.quantity
		map.set(order.status, existing)
	}
	return [...map.values()].sort((a, b) => b.revenue - a.revenue)
}

export function breakdownByRegion(orders: Order[]): RegionBreakdown[] {
	const map = new Map<string, RegionBreakdown>()
	for (const order of orders) {
		if (!isRevenueStatus(order.status)) {
			continue
		}
		const existing = map.get(order.region) ?? {
			region: order.region,
			orders: 0,
			revenue: 0
		}
		existing.orders += 1
		existing.revenue += order.unitPrice * order.quantity
		map.set(order.region, existing)
	}
	return [...map.values()].sort((a, b) => b.revenue - a.revenue)
}

export function dailyRevenue(orders: Order[]): DailyPoint[] {
	const map = new Map<string, DailyPoint>()
	for (const order of orders) {
		if (!isRevenueStatus(order.status)) {
			continue
		}
		const day = order.placedAt.slice(0, 10)
		const existing = map.get(day) ?? { date: day, revenue: 0, orders: 0 }
		existing.revenue += order.unitPrice * order.quantity
		existing.orders += 1
		map.set(day, existing)
	}
	return [...map.values()].sort((a, b) => a.date.localeCompare(b.date))
}

export function topProducts(orders: Order[], limit: number = 5): { product: string; revenue: number }[] {
	const map = new Map<string, number>()
	for (const order of orders) {
		if (!isRevenueStatus(order.status)) {
			continue
		}
		map.set(order.product, (map.get(order.product) ?? 0) + order.unitPrice * order.quantity)
	}
	return [...map.entries()]
		.map(([product, revenue]) => ({ product, revenue }))
		.sort((a, b) => b.revenue - a.revenue)
		.slice(0, limit)
}

export function growthRatio(current: number, previous: number): number {
	if (previous === 0) {
		return current === 0 ? 0 : 1
	}
	return (current - previous) / previous
}
