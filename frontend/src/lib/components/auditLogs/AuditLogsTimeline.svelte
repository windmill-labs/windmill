<script lang="ts">
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
		type ChartData,
		type ChartOptions
	} from 'chart.js'
	import type { AuditLog } from '$lib/gen'
	import { Scatter } from '../chartjs-wrappers/chartJs'
	import { createEventDispatcher } from 'svelte'

	let { logs = [] }: { logs: AuditLog[] } = $props()

	const dispatch = createEventDispatcher()

	// Register ChartJS components
	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		zoomPlugin,
		LineElement,
		CategoryScale,
		LinearScale,
		PointElement,
		TimeScale
	)

	function addSeconds(date: Date, seconds: number): Date {
		date.setTime(date.getTime() + seconds * 1000)
		return date
	}

	const zoomOptions = {
		pan: {
			enabled: true,
			modifierKey: 'ctrl' as 'ctrl',
			onPanComplete: ({ chart }) => {
				dispatch('zoom', {
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		},
		zoom: {
			drag: {
				enabled: true
			},
			mode: 'x' as 'x',
			onZoom: ({ chart }) => {
				dispatch('zoom', {
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		}
	}
	// Color mapping for different action kinds
	const actionColors = {
		Execute: '#3b82f6', // blue
		Delete: '#ef4444', // red
		Update: '#eab308', // yellow
		Create: '#22c55e', // green
		default: '#6b7280' // gray
	}

	function getActionColor(actionKind: string): string {
		return actionColors[actionKind as keyof typeof actionColors] || actionColors.default
	}

	function groupLogsBySpan(logs: AuditLog[]): Record<string, AuditLog[]> {
		const grouped: Record<string, AuditLog[]> = {}

		const jobGrouped: Map<string, AuditLog[]> = new Map()
		for (const log of logs) {
			const spanId = log.span || 'untraced'
			if (spanId.startsWith('job-span-')) {
				const jobid = spanId.slice('job-span-'.length)

				if (!jobGrouped.has(jobid)) {
					jobGrouped.set(jobid, [])
				}
				jobGrouped.get(jobid)?.push(log)
				continue
			}
			if (!grouped[spanId]) {
				grouped[spanId] = []
			}
			grouped[spanId].push(log)
		}

		for (const jobid of jobGrouped.keys()) {
			const auditSpan = Object.values(grouped).flat().find((log) => log.parameters?.uuid === jobid)?.span
			if (auditSpan != undefined) {
				grouped[auditSpan].push(...(jobGrouped.get(jobid)!))
			} else {
				if (!grouped[jobid]) {
					grouped[jobid] = []
				}
				grouped[jobid].push(...jobGrouped.get(jobid)!)
			}

		}
		// Sort logs within each span by timestamp
		Object.values(grouped).forEach((spanLogs) => {
			spanLogs.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())
		})

		return grouped
	}

	function isDark(): boolean {
		return document.documentElement.classList.contains('dark')
	}

	// Set chart defaults based on theme
	ChartJS.defaults.color = isDark() ? '#ccc' : '#666'
	ChartJS.defaults.borderColor = isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'

	const groupedLogs = $derived(groupLogsBySpan(logs))
	const spanIds = $derived(Object.keys(groupedLogs).sort())

	// Transform data for ChartJS scatter plot
	const chartData = $derived((): ChartData<'scatter'> => {
		if (logs.length === 0) {
			return { datasets: [] }
		}

		const datasets = spanIds.map((spanId, index) => {
			const spanLogs = groupedLogs[spanId]
			console.log(`Processing span ${spanId} with ${spanLogs.length} logs`)
			return {
				label: spanId === 'untraced' ? 'Untraced' : spanId,
				data: spanLogs.map((log) => ({
					x: log.timestamp as any,
					y: index, // Each span gets its own y-axis position
					log: log // Store full log data for tooltips
				})) as any[],
				backgroundColor: spanLogs.map((log) => getActionColor(log.action_kind)),
				borderColor: spanLogs.map((log) => getActionColor(log.action_kind)),
				borderWidth: 2,
				pointRadius: 6,
				pointHoverRadius: 8,
				showLine: false
			}
		})
		console.log('Chart datasets:', datasets)
		return { datasets }
	})

	const chartOptions = $derived(
		(): ChartOptions<'scatter'> =>
			({
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					zoom: zoomOptions,
					legend: {
						display: false // We'll create our own legend
					},
					tooltip: {
						callbacks: {
							title: function (context: any) {
								const log = context[0].raw.log
								return `${log.operation} - ${log.action_kind}`
							},
							label: function (context: any) {
								const log = context.raw.log
								return [
									`User: ${log.username}`,
									`Resource: ${log.resource}`,
									`Time: ${new Date(log.timestamp).toLocaleString()}`,
									`Span: ${log.span || log.span || 'untraced'}`
								]
							}
						}
					}
				},
				scales: {
					x: {
						type: 'time',
						time: {
							displayFormats: {
								millisecond: 'HH:mm:ss.SSS',
								second: 'HH:mm:ss',
								minute: 'HH:mm',
								hour: 'MMM dd HH:mm',
								day: 'MMM dd',
								week: 'MMM dd',
								month: 'MMM yyyy',
								quarter: 'MMM yyyy',
								year: 'yyyy'
							}
						},
						title: {
							display: true,
							text: 'Time'
						},
						grid: {
							display: true,
							color: isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
						}
					},
					y: {
						type: 'linear',
						min: spanIds.length > 0 ? -0.5 : 0,
						max: spanIds.length > 0 ? spanIds.length - 0.5 : 1,
						ticks: {
							stepSize: 1,
							callback: function (value: any) {
								const index = Math.round(value)
								if (index >= 0 && index < spanIds.length) {
									const spanId = spanIds[index]
									return spanId === 'untraced' ? 'Untraced' : spanId
								}
								return ''
							}
						},
						title: {
							display: true,
							text: 'Span ID'
						},
						grid: {
							display: true,
							color: isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
						}
					}
				},
				onClick: (event: any, elements: any) => {
					if (elements.length > 0) {
						const element = elements[0]
						const log = (chartData().datasets[element.datasetIndex].data[element.index] as any).log
						dispatch('logSelected', log)
					}
				},
				animation: {
					duration: 300
				}
			}) as any
	)

	function unifySpanForSameJobLogs(logs: AuditLog[]): AuditLog[] {
		return logs
	}
</script>

<div class="timeline-container p-4 bg-surface mb-4">
	<h3 class="text-lg font-semibold mb-4">Audit Logs Timeline</h3>

	{#if logs.length === 0}
		<div class="text-center py-8 text-secondary"> No audit logs to display </div>
	{:else}
		<!-- Chart container -->
		<div class="h-80">
			<Scatter data={chartData()} options={chartOptions()} />
		</div>

		<!-- Legend -->
		<div class="flex items-center gap-4 mt-4 pt-4 border-t">
			<span class="text-sm text-secondary">Actions:</span>
			<div class="flex gap-3 flex-wrap">
				<div class="flex items-center gap-1">
					<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Create}"></div>
					<span class="text-xs">Create</span>
				</div>
				<div class="flex items-center gap-1">
					<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Update}"></div>
					<span class="text-xs">Update</span>
				</div>
				<div class="flex items-center gap-1">
					<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Execute}"></div>
					<span class="text-xs">Execute</span>
				</div>
				<div class="flex items-center gap-1">
					<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Delete}"></div>
					<span class="text-xs">Delete</span>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.timeline-container {
		max-height: 500px;
		overflow-y: auto;
	}
</style>
