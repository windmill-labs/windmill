// Thin wrappers around the app's backend runnables. Centralizing the calls
// here keeps the components free of `backend.*` plumbing and gives one place to
// normalize the request/response shapes.

import { backend } from 'wmill'
import type { Order, MetricCardData } from '../data/seedData'

export interface DateRange {
	from: string
	to: string
}

export interface MetricsResponse {
	cards: MetricCardData[]
	generatedAt: string
}

export interface OrdersResponse {
	orders: Order[]
	total: number
}

export interface SummaryResponse {
	totalRevenue: number
	netRevenue: number
	totalOrders: number
	averageOrderValue: number
	unitsSold: number
	refundedRevenue: number
	currency: string
}

export async function fetchMetrics(range: DateRange, region: string): Promise<MetricsResponse> {
	return backend.loadMetrics({ from: range.from, to: range.to, region })
}

export async function fetchOrders(
	range: DateRange,
	region: string,
	status: string
): Promise<OrdersResponse> {
	return backend.loadOrders({
		from: range.from,
		to: range.to,
		region,
		status
	})
}

export async function fetchSummary(range: DateRange, region: string): Promise<SummaryResponse> {
	return backend.computeSummary({ from: range.from, to: range.to, region })
}

export async function requestExport(
	range: DateRange,
	region: string,
	format: 'csv' | 'json'
): Promise<{ url: string; rows: number }> {
	return backend.exportReport({ from: range.from, to: range.to, region, format })
}

export function defaultRange(): DateRange {
	return { from: '2024-05-01', to: '2024-05-31' }
}

export function rangeForPreset(preset: string): DateRange {
	switch (preset) {
		case '7d':
			return { from: '2024-05-25', to: '2024-05-31' }
		case '14d':
			return { from: '2024-05-18', to: '2024-05-31' }
		case '30d':
			return { from: '2024-05-01', to: '2024-05-31' }
		case 'qtd':
			return { from: '2024-04-01', to: '2024-05-31' }
		default:
			return defaultRange()
	}
}
