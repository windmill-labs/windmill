<script lang="ts">
	import { PERSIST_JOB_METRICS_SETTING } from '$lib/consts'
	import { type MetricDataPoint, SettingService } from '$lib/gen'
	import { displayTime } from '$lib/utils'
	import { enterpriseLicense, userStore } from '$lib/stores'
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
	import { Line } from 'svelte-chartjs'
	import { Alert } from './common'

	ChartJS.register(Title, Tooltip, Legend, LineElement, LinearScale, PointElement, CategoryScale)

	let jobMetricsEnabled: boolean | undefined = undefined
	export let jobStats: MetricDataPoint[] | undefined = undefined

	let data: {
		x: number
		y: number
	}[] = []
	let labels: string[] = []
	async function computeData() {
		if (jobMetricsEnabled === undefined) {
			await checkJobMetricsEnabled()
		}
		for (let dp of jobStats ?? []) {
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

	async function checkJobMetricsEnabled() {
		let setting_value = await SettingService.getGlobal({ key: PERSIST_JOB_METRICS_SETTING })
		console.log('persist_job_metrics', setting_value)
		jobMetricsEnabled = setting_value ?? true
	}

	$: checkJobMetricsEnabled()
	$: jobStats && computeData()
</script>

<div class="relative max-h-100">
	{#if !$enterpriseLicense}
		<Alert type="error" title="Enterprise Edition only feature">
			Job metrics are only available on Windmill Enterprise Edition.
		</Alert>
	{:else if !(jobMetricsEnabled ?? true) && $userStore?.is_super_admin}
		<Alert type="warning" title="Not seeing any metrics?">
			Job metrics need to be enabled. Go to the <a href="#superadmin-settings"
				>Windmill Instance Settings</a
			> Debug tab and toggle "Persist job metrics" on.
		</Alert>
	{:else if !(jobMetricsEnabled ?? true) && (jobStats?.length ?? 0) === 0}
		<Alert type="warning" title="Not seeing any metrics?">
			Job metrics need to be enabled by a Windmill Administrator.
		</Alert>
	{:else if (jobStats?.length ?? 0) === 0}
		<Alert type="info" title="No metric available">No metric available for this job.</Alert>
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
