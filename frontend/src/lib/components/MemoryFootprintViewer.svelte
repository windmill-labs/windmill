<script lang="ts">
	import { type MetricDataPoint, MetricsService } from '$lib/gen'
	import { displayTime } from '$lib/utils'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import {
		CategoryScale,
		Chart as ChartJS,
		Legend,
		LinearScale,
		LineElement,
		PointElement,
		Title,
		Tooltip
	} from 'chart.js'
	import { Line } from '$lib/components/chartjs-wrappers/chartJs'
	import { Alert } from './common'

	ChartJS.register(Title, Tooltip, Legend, LineElement, LinearScale, PointElement, CategoryScale)

	export let jobId: string
	export let jobUpdateLastFetch: Date | undefined

	let jobMetricsLastFetch: Date | undefined = undefined
	let jobMemoryStats: MetricDataPoint[] | undefined = undefined

	let data: {
		x: number
		y: number
	}[] = []
	let labels: string[] = []

	async function loadMetricsData() {
		try {
			let jobStatsPromise = MetricsService.getJobMetrics({
				workspace: $workspaceStore!,
				id: jobId,
				requestBody: {
					from_timestamp: jobMetricsLastFetch?.toISOString(),
					timeseries_max_datapoints: 0
				}
			})
			jobMetricsLastFetch = new Date()
			let jobStats = await jobStatsPromise

			let memoryTimeseries =
				jobStats.timeseries_metrics?.filter((ts) => ts.metric_id === 'memory_kb') ?? []

			if (memoryTimeseries.length > 0) {
				jobMemoryStats = (jobMemoryStats ?? []).concat(memoryTimeseries[0].values)
			}
		} catch {
			console.error('Unable to load metrics data for job', jobId)
			return
		}

		for (let dp of jobMemoryStats ?? []) {
			let ts = new Date(dp.timestamp).valueOf()
			if (data.length === 0 || ts > data[data.length - 1].x) {
				data.push({
					x: ts,
					y: dp.value
				})
				labels.push(displayTime(dp.timestamp))
			}
		}
		data = [...data]
	}

	$: jobUpdateLastFetch && loadMetricsData()
</script>

<div class="relative max-h-100">
	{#if !$enterpriseLicense}
		<Alert type="error" title="Enterprise Edition only feature">
			Job metrics are only available on Windmill Enterprise Edition.
		</Alert>
	{:else if (jobMemoryStats?.length ?? 0) === 0}
		<Alert type="info" title="No metric available"
			>No data points available for this job. Metrics are recorded only for jobs running for more
			than 500ms.</Alert
		>
	{/if}

	<Line
		class="w-full max-h-80"
		data={{
			labels: labels,
			datasets: [
				{
					label: 'Job memory footprint (kB)',
					data: data,
					fill: false,
					borderColor: 'rgb(59, 130, 246, 0.8)',
					backgroundColor: 'rgb(59, 130, 246, 0.8)',
					tension: 0.1
				}
			]
		}}
		options={{
			animation: {
				duration: 0
			},
			maintainAspectRatio: false,
			responsive: true
		}}
	/>
</div>
