<script lang="ts">
	import { Bar } from 'svelte-chartjs'
	import 'chartjs-adapter-date-fns'
	import zoomPlugin from 'chartjs-plugin-zoom'
	import {
		Chart as ChartJS,
		Title,
		Tooltip,
		Legend,
		LineElement,
		CategoryScale,
		LinearScale,
		PointElement,
		TimeScale,
		LogarithmicScale,
		BarElement
	} from 'chart.js'
	import type { AuditLog } from '$lib/gen'

	export let logs: AuditLog[] | undefined = []

	$: groupedLogs =
		logs?.reverse().reduce((acc, log) => {
			const date = new Date(log.timestamp)
			const dateKey = date.toLocaleDateString()
			if (!acc[dateKey]) {
				acc[dateKey] = []
			}
			acc[dateKey].push(log)
			return acc
		}, {} as Record<string, AuditLog[]>) ?? {}

	$: barOptions = {
		responsive: true,
		maintainAspectRatio: false,
		animation: {
			duration: 0
		},
		plugins: {
			legend: {
				display: false
			}
		},
		scales: {
			x: {
				display: false,
				stacked: true
			},
			y: {
				display: false,
				stacked: true
			}
		},
		barThickness: 6,
		barPercentage: 0.4
	}

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		zoomPlugin,
		LineElement,
		CategoryScale,
		LinearScale,
		LogarithmicScale,
		PointElement,
		TimeScale,
		BarElement
	)

	$: data = {
		labels: Object.keys(groupedLogs),
		datasets: [
			{
				label: '% of executions',
				data: Object.values(groupedLogs).map(
					(logs) => logs.filter((log) => log.action_kind === 'Execute').length
				),
				backgroundColor: ['#60a5fa'],
				borderWidth: 0
			},
			{
				label: '% of creation',
				data: Object.values(groupedLogs).map(
					// @ts-ignore
					(logs) => logs.filter((log) => log.action_kind === 'Create').length
				),
				backgroundColor: ['#4ade80'],
				borderWidth: 0
			},
			{
				label: '% of updates',
				data: Object.values(groupedLogs).map(
					// @ts-ignore
					(logs) => logs.filter((log) => log.action_kind === 'Update').length
				),
				backgroundColor: ['#facc15'],
				borderWidth: 0
			},
			{
				label: '% of deletions',
				data: Object.values(groupedLogs).map(
					(logs) => logs.filter((log) => log.action_kind === 'Delete').length
				),
				backgroundColor: ['#f87171'],
				borderWidth: 0
			}
		]
	}
</script>

<Bar {data} options={barOptions} />
