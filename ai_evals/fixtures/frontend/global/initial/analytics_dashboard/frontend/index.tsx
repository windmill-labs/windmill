import React, { useEffect, useMemo, useState } from 'react'
import { Sidebar, type DashboardView } from './components/Sidebar'
import { FilterBar } from './components/FilterBar'
import { MetricGrid } from './components/MetricGrid'
import { SummaryPanel } from './components/SummaryPanel'
import { RevenueChart } from './components/RevenueChart'
import { OrdersTable } from './components/OrdersTable'
import { RegionTable } from './components/RegionTable'
import { TopProducts } from './components/TopProducts'
import { EmptyState } from './components/EmptyState'
import { fetchMetrics, fetchOrders, rangeForPreset, type DateRange } from './lib/api'
import {
	seedOrders,
	seedMetricCards,
	ordersInRange,
	ordersForRegion,
	type Order,
	type MetricCardData
} from './data/seedData'

const App = () => {
	const [view, setView] = useState<DashboardView>('overview')
	const [preset, setPreset] = useState('30d')
	const [range, setRange] = useState<DateRange>(rangeForPreset('30d'))
	const [region, setRegion] = useState('all')
	const [status, setStatus] = useState('all')

	const [metrics, setMetrics] = useState<MetricCardData[]>(seedMetricCards)
	const [orders, setOrders] = useState<Order[]>(seedOrders)
	const [loadingMetrics, setLoadingMetrics] = useState(true)
	const [loadingOrders, setLoadingOrders] = useState(true)
	const [errored, setErrored] = useState(false)

	useEffect(() => {
		let cancelled = false
		setLoadingMetrics(true)
		fetchMetrics(range, region)
			.then((result) => {
				if (!cancelled) {
					setMetrics(result.cards)
				}
			})
			.catch(() => {
				if (!cancelled) {
					setMetrics(seedMetricCards)
				}
			})
			.finally(() => {
				if (!cancelled) {
					setLoadingMetrics(false)
				}
			})
		return () => {
			cancelled = true
		}
	}, [range, region])

	useEffect(() => {
		let cancelled = false
		setLoadingOrders(true)
		setErrored(false)
		fetchOrders(range, region, status)
			.then((result) => {
				if (!cancelled) {
					setOrders(result.orders)
				}
			})
			.catch(() => {
				if (!cancelled) {
					// Fall back to the bundled seed data so the dashboard still renders.
					const scoped = ordersForRegion(
						ordersInRange(seedOrders, range.from, range.to),
						region
					).filter((order) => status === 'all' || order.status === status)
					setOrders(scoped)
					setErrored(true)
				}
			})
			.finally(() => {
				if (!cancelled) {
					setLoadingOrders(false)
				}
			})
		return () => {
			cancelled = true
		}
	}, [range, region, status])

	const handlePresetChange = (nextPreset: string, nextRange: DateRange) => {
		setPreset(nextPreset)
		setRange(nextRange)
	}

	// Orders that drive the summary/chart panels — the table applies the status
	// filter itself, so the panels see the same range/region scoped orders.
	const scopedOrders = useMemo(() => orders, [orders])

	const renderView = () => {
		switch (view) {
			case 'orders':
				return <OrdersTable orders={scopedOrders} loading={loadingOrders} />
			case 'regions':
				return <RegionTable orders={scopedOrders} />
			case 'products':
				return <TopProducts orders={scopedOrders} limit={8} />
			case 'overview':
			default:
				return (
					<div className="space-y-6">
						<MetricGrid metrics={metrics} loading={loadingMetrics} />
						<SummaryPanel orders={scopedOrders} loading={loadingOrders} />
						<div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
							<RevenueChart orders={scopedOrders} />
							<TopProducts orders={scopedOrders} />
						</div>
						<RegionTable orders={scopedOrders} />
					</div>
				)
		}
	}

	return (
		<div className="flex h-screen bg-gray-100 text-gray-900">
			<Sidebar active={view} onSelect={setView} />
			<div className="flex flex-1 flex-col overflow-hidden">
				<header className="border-b border-gray-200 bg-white px-6 py-5">
					<p className="text-xs font-semibold uppercase tracking-wide text-indigo-500">
						Acme Inc
					</p>
					<h1 className="text-2xl font-bold text-gray-900">Operations Console</h1>
					<p className="mt-1 text-sm text-gray-500">
						Revenue, orders, and regional performance at a glance.
					</p>
				</header>
				<FilterBar
					region={region}
					status={status}
					preset={preset}
					range={range}
					onRegionChange={setRegion}
					onStatusChange={setStatus}
					onPresetChange={handlePresetChange}
				/>
				<main className="flex-1 overflow-auto p-6">
					{errored ? (
						<div className="mb-4 rounded-lg border border-amber-200 bg-amber-50 px-4 py-2 text-sm text-amber-700">
							Showing locally bundled data — the live feed is unavailable.
						</div>
					) : null}
					{scopedOrders.length === 0 && !loadingOrders ? (
						<EmptyState
							title="Nothing to show yet"
							description="No data for the selected range, region, and status."
						/>
					) : (
						renderView()
					)}
				</main>
			</div>
		</div>
	)
}

export default App
