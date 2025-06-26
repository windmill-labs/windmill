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
	import { Loader2 } from 'lucide-svelte'
	import { untrack } from 'svelte'
	import { sleep } from '$lib/utils'
	import { usePromise } from '$lib/svelte5Utils.svelte'

	interface Props {
		logs: AuditLog[]
		minTimeSet: string | undefined
		maxTimeSet: string | undefined
		onMissingJobSpan?: (jobId: string, jobLogs: AuditLog[]) => Promise<AuditLog[]>
		onZoom?: (range: { min: Date; max: Date }) => void
		onLogSelected?: (log: any) => void
	}

	let {
		logs = [],
		minTimeSet,
		maxTimeSet,
		onMissingJobSpan,
		onZoom,
		onLogSelected
	}: Props = $props()

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
			mode: 'x',
			enabled: true,
			modifierKey: 'ctrl',
			onPanComplete: ({ chart }) => {
				chartInstance = chart
				onZoom?.({
					min: addSeconds(new Date(chart.scales.x.min), -1),
					max: addSeconds(new Date(chart.scales.x.max), 1)
				})
			}
		},
		zoom: {
			drag: {
				enabled: true
			},
			mode: 'x',
			scaleMode: 'y',
			onZoom: ({ chart }) => {
				chartInstance = chart
				// zoomTrigger++ // Trigger recalculation of jittering
				onZoom?.({
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

	async function groupLogsBySpan(
		logs: AuditLog[],
		onMissingJobSpan?: (jobId: string, jobLogs: AuditLog[]) => Promise<AuditLog[]>
	): Promise<{
		grouped: Record<string, AuditLog[]>
		jobGrouped: Map<string, AuditLog[]>
	}> {
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
			const j = Object.values(grouped)
				.flat()
				.find((log) => log.parameters?.uuid === jobid)
			if (j?.span != undefined) {
				grouped[j.span].push(...jobGrouped.get(jobid)!)
				jobGrouped.get(jobid)?.push(j)
			} else {
				// Try to fetch missing job execution audit log
				if (onMissingJobSpan) {
					try {
						const jobLogs = jobGrouped.get(jobid)!
						const additionalLogs = await onMissingJobSpan(jobid, jobLogs)

						// Look for the job execution audit log in the new results
						const jobExecutionLog = additionalLogs.find((log) => log.parameters?.uuid === jobid)
						if (jobExecutionLog?.span) {
							if (!grouped[jobExecutionLog.span]) {
								grouped[jobExecutionLog.span] = []
							}
							grouped[jobExecutionLog.span].push(jobExecutionLog, ...jobLogs)
							jobGrouped.get(jobid)?.push(jobExecutionLog)
							continue
						}
					} catch (error) {
						console.warn(`Failed to fetch missing job audit span for job ${jobid}:`, error)
					}
				}

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

		return { grouped, jobGrouped }
	}

	function isDark(): boolean {
		return document.documentElement.classList.contains('dark')
	}

	// Function to apply zoom-aware jittering to overlapping points
	function applyJittering(dataPoints: any[], baseY: number, chartInstance?: any): any[] {
		if (dataPoints.length <= 1) return dataPoints

		// Sort by timestamp
		const sorted = [...dataPoints].sort((a, b) => new Date(a.x).getTime() - new Date(b.x).getTime())

		// Calculate visual overlap based on chart scale
		const pointRadius = 0.8 // Current point radius
		const overlapThreshold = pointRadius * 2 // Points overlap if closer than this in pixels

		// Group points that visually overlap
		const groups: any[][] = []
		let currentGroup: any[] = [sorted[0]]

		for (let i = 1; i < sorted.length; i++) {
			const prevTime = new Date(sorted[i - 1].x).getTime()
			const currTime = new Date(sorted[i].x).getTime()

			// Calculate pixel distance between points
			let pixelDistance = overlapThreshold + 1 // Default to no overlap

			if (chartInstance && chartInstance.scales && chartInstance.scales.x) {
				const prevPixel = chartInstance.scales.x.getPixelForValue(prevTime)
				const currPixel = chartInstance.scales.x.getPixelForValue(currTime)
				pixelDistance = Math.abs(currPixel - prevPixel)
			} else {
				// Fallback: estimate based on time difference and typical zoom level
				const timeDiff = currTime - prevTime
				// Assume roughly 1 pixel per 100ms at default zoom
				pixelDistance = timeDiff / 100
			}

			if (pixelDistance < overlapThreshold) {
				currentGroup.push(sorted[i])
			} else {
				groups.push(currentGroup)
				currentGroup = [sorted[i]]
			}
		}
		groups.push(currentGroup)

		const jitteredPoints: any[] = []
		groups.forEach((group) => {
			if (group.length === 1) {
				jitteredPoints.push({
					...group[0],
					y: baseY,
					isCluster: false,
					clusterSize: 1
				})
			} else {
				const jitterRange = 0.4

				group.forEach((point, index) => {
					let jitterOffset =
						(1 - Math.exp(-group.length / 50)) * jitterRange * (Math.random() - 0.5)

					jitteredPoints.push({
						...point,
						y: baseY + jitterOffset,
						originalY: baseY,
						isCluster: false,
						clusterSize: group.length,
						clusterIndex: index
					})
				})
			}
		})

		return jitteredPoints
	}

	// Set chart defaults based on theme
	ChartJS.defaults.color = isDark() ? '#ccc' : '#666'
	ChartJS.defaults.borderColor = isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'

	async function getGroupedData(): Promise<{
		grouped: Record<string, AuditLog[]>
		jobGrouped: Map<string, AuditLog[]>
	}> {
	console.log("Getting grouped data")
		if (logs.length === 0) {
			await sleep(1)
			return { grouped: {}, jobGrouped: new Map() }
		}

		try {
			return await groupLogsBySpan(logs, onMissingJobSpan)
		} catch (error) {
			console.error('Error grouping logs:', error)
			// Fallback to sync grouping without missing job span resolution
			return await groupLogsBySpan(logs)
		}
	}

	let groupedData = usePromise(getGroupedData)
	// let isGrouping = $state(false)
	let chartInstance: any = null
	let zoomTrigger = $state(0)

	let { minTime, maxTime } = $derived(computeMinMaxTime(logs, minTimeSet, maxTimeSet))

	function computeMinMaxTime(
		logs: AuditLog[] | undefined,
		minTimeSet: string | undefined,
		maxTimeSet: string | undefined
	) {
		let minTime = addSeconds(new Date(), -300)
		let maxTime = new Date()

		let minTimeSetDate = minTimeSet ? new Date(minTimeSet) : undefined
		let maxTimeSetDate = maxTimeSet ? new Date(maxTimeSet) : undefined
		if (minTimeSetDate && maxTimeSetDate) {
			minTime = minTimeSetDate
			maxTime = maxTimeSetDate
			return { minTime, maxTime }
		}

		if (logs == undefined || logs?.length == 0) {
			minTime = minTimeSetDate ?? addSeconds(new Date(), -300)
			maxTime = maxTimeSetDate ?? new Date()
			return { minTime, maxTime }
		}

		const maxLogsTime = new Date(
			logs.reduce((max, current) =>
				new Date(current.timestamp) > new Date(max.timestamp) ? current : max
			).timestamp
		)
		const maxJob = maxTimeSetDate === undefined ? new Date() : maxLogsTime

		const minJob = new Date(
			logs.reduce((max, current) =>
				new Date(current.timestamp) < new Date(max.timestamp) ? current : max
			).timestamp
		)

		const diff = (maxJob.getTime() - minJob.getTime()) / 20000

		minTime = minTimeSetDate ?? addSeconds(minJob, -diff)
		if (maxTimeSetDate) {
			maxTime = maxTimeSetDate ?? maxJob
		} else {
			maxTime = maxTimeSetDate ?? addSeconds(maxJob, diff)
		}
		return { minTime, maxTime }
	}

	$effect(() => {
		logs && untrack(() => groupedData.refresh())
	})

	// Reactive grouping that handles async operations
	// $effect(async () => {
	// 	if (logs.length === 0) {
	// 		await sleep(1)
	// 		groupedData = { grouped: {}, jobGrouped: new Map() }
	// 		return
	// 	}
	//
	// 	isGrouping = true
	// 	try {
	// 		groupedData = await groupLogsBySpan(logs, onMissingJobSpan)
	// 	} catch (error) {
	// 		console.error('Error grouping logs:', error)
	// 		// Fallback to sync grouping without missing job span resolution
	// 		groupedData = await groupLogsBySpan(logs)
	// 	} finally {
	// 		isGrouping = false
	// 	}
	// })

	const groupedLogs = $derived(groupedData.value?.grouped ?? {})
	const jobGrouped = $derived(groupedData.value?.jobGrouped ?? new Map())
	const spanIds = $derived(Object.keys(groupedLogs).sort())
	// const spanAuthors = $derived(
	// 	spanIds.map((span) => {
	// 		const endUser = groupedLogs[span][0]?.parameters?.end_user
	// 		const endUserText = endUser ? ` (${endUser})` : ''
	// 		return groupedLogs[span]?.length > 0 ? `${groupedLogs[span][0].username}${endUserText}` : ''
	// 	})
	// )

	// Transform data for ChartJS scatter plot
	const chartData = $derived((): ChartData<'scatter'> => {
		if (untrack(() => logs.length) === 0) {
			return { datasets: [] }
		}

		// Include zoomTrigger in dependencies to recalculate on zoom
		// const _ = zoomTrigger

		const datasets: any[] = []

		// Create datasets for regular span groups (points only)
		spanIds.forEach((spanId, index) => {
			const spanLogs = groupedLogs[spanId]

			// Create initial data points
			const dataPoints = spanLogs.map((log) => ({
				x: log.timestamp as any,
				y: index, // Each span gets its own y-axis position
				log: log // Store full log data for tooltips
			}))

			// Apply zoom-aware jittering to spread out overlapping points
			const jitteredPoints = applyJittering(dataPoints, index, chartInstance)
			// const jitteredPoints = dataPoints

			datasets.push({
				label: spanId === 'untraced' ? 'Untraced' : spanId,
				data: jitteredPoints,
				backgroundColor: jitteredPoints.map((point) => {
					const baseColor = getActionColor(point.log.action_kind)
					// Make clustered points slightly more opaque
					return point.isCluster ? baseColor + 'E0' : baseColor
				}),
				borderColor: jitteredPoints.map((point) => {
					const baseColor = getActionColor(point.log.action_kind)
					// Add white border to clustered points for better visibility
					return point.isCluster ? '#ffffff' : baseColor
				}),
				borderWidth: jitteredPoints.map((point) => (point.isCluster ? 1 : 1)),
				pointRadius: jitteredPoints.map((point) => (point.isCluster ? 3 : 3)),
				pointHoverRadius: jitteredPoints.map((point) => (point.isCluster ? 5 : 5)),
				showLine: false
			})
		})

		// Create datasets for job-connected lines
		jobGrouped.forEach((jobLogs: AuditLog[], jobId: string) => {
			if (jobLogs.length > 1) {
				// Only create lines if there are multiple points
				// Sort job logs by timestamp to ensure proper line connection
				const sortedJobLogs = [...jobLogs].sort(
					(a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
				)

				// Find the y-position for each log based on its span and jittered position
				const lineData = sortedJobLogs.map((log) => {
					const spanId = log.span || 'untraced'
					let baseYPosition = spanIds.indexOf(spanId)
					if (baseYPosition === -1) {
						// Handle job-span logs that might not be in regular spans
						const jobSpanId = spanId.startsWith('job-span-')
							? spanId.slice('job-span-'.length)
							: spanId
						baseYPosition = spanIds.findIndex((id) => id === jobSpanId)
						if (baseYPosition === -1) {
							// If still not found, assign to the span where this job's audit logs are grouped
							const auditSpan = Object.entries(groupedLogs).find(([, spanLogs]) =>
								spanLogs.some((l) => l.parameters?.uuid === jobId)
							)?.[0]
							baseYPosition = auditSpan ? spanIds.indexOf(auditSpan) : 0
						}
					}

					// Find the jittered position for this specific log
					let jitteredY = baseYPosition
					if (baseYPosition >= 0 && baseYPosition < datasets.length) {
						const spanDataset = datasets[baseYPosition]
						if (spanDataset && spanDataset.data) {
							const matchingPoint = spanDataset.data.find(
								(point: any) => point.log && point.log.id === log.id
							)
							if (matchingPoint) {
								jitteredY = matchingPoint.y
							}
						}
					}

					return {
						x: log.timestamp as any,
						y: jitteredY,
						log: log
					}
				})

				datasets.push({
					label: `Job ${jobId} Connection`,
					data: lineData,
					backgroundColor: 'transparent',
					borderColor: '#8b5cf6', // Purple color for job connections
					borderWidth: 2,
					pointRadius: 0, // Hide points for connection lines
					pointHoverRadius: 0,
					showLine: true,
					tension: 0, // Straight lines
					fill: false
				})
			}
		})
		console.log("Completed data!")
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
								const point = context[0].raw
								let title = `${log.operation} - ${log.action_kind}`
								// if (point.isCluster) {
								// 	title += ` (${point.clusterIndex + 1} of ${point.clusterSize})`
								// }
								return title
							},
							label: function (context: any) {
								const log = context.raw.log
								const point = context.raw
								const labels = [
									`User: ${log.username}`,
									`Resource: ${log.resource}`,
									`Time: ${new Date(log.timestamp).toLocaleString()}`
									// `Span: ${log.span || log.span || 'untraced'}`
								]
								// if (point.isCluster) {
								// 	labels.push(`Clustered with ${point.clusterSize - 1} other logs`)
								// }
								return labels
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
							display: false,
							color: isDark() ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)'
						},
						ticks: {
							stepSize: 1
						},
						min: minTime,
						max: maxTime
					},
					y: {
						type: 'linear',
						// min: spanIds.length > 0 ? 0 : 0,
						// max: spanIds.length > 0 ? spanIds.length : 1,
						ticks: {
							stepSize: 1,
							callback: function (value: any) {
								const index = Math.round(value)
								// console.log(index, value)
								if (index >= 0 && index < spanIds.length) {
									const spanId = spanIds[index]
									return spanId === 'untraced' ? 'Untraced' : spanId.slice(0, 30)
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
				onClick: (event: any, elements: any, chart: any) => {
					// Capture chart instance for jittering calculations
					if (!chartInstance) {
						chartInstance = chart
					}
					if (elements.length > 0) {
						const element = elements[0]
						const log = (chartData().datasets[element.datasetIndex].data[element.index] as any).log
						onLogSelected?.(log)
					}
				},
				onHover: (event: any, elements: any, chart: any) => {
					// Capture chart instance for jittering calculations
					if (!chartInstance) {
						chartInstance = chart
					}
				},
				animation: {
					duration: 300
				}
			}) as any
	)
</script>

<div class="timeline-container p-4 bg-surface mb-4">
	<div class="flex items-center gap-2 mb-4">
		<h3 class="text-lg font-semibold">Audit Logs Timeline</h3>
		{#if groupedData.status === 'loading'}
			<Loader2 size={16} class="animate-spin text-secondary" />
			<span class="text-sm text-secondary">Resolving job connections...</span>
		{/if}
	</div>

	{#if logs.length === 0}
		<div class="text-center py-8 text-secondary"> No audit logs to display </div>
	{:else if !groupedData}
		<div class="text-center py-8 text-secondary">
			<Loader2 size={24} class="animate-spin mx-auto mb-2" />
			Processing audit logs...
		</div>
	{:else}
		<!-- Chart container -->
		<div class="h-80">
			<Scatter data={chartData()} options={chartOptions()} />
		</div>

		<!-- Legend -->
		<!-- <div class="flex items-center gap-4 mt-4 pt-4 border-t"> -->
		<!-- 	<span class="text-sm text-secondary">Actions:</span> -->
		<!-- 	<div class="flex gap-3 flex-wrap"> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Create}"></div> -->
		<!-- 			<span class="text-xs">Create</span> -->
		<!-- 		</div> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Update}"></div> -->
		<!-- 			<span class="text-xs">Update</span> -->
		<!-- 		</div> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Execute}"></div> -->
		<!-- 			<span class="text-xs">Execute</span> -->
		<!-- 		</div> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div class="w-3 h-3 rounded-full" style="background-color: {actionColors.Delete}"></div> -->
		<!-- 			<span class="text-xs">Delete</span> -->
		<!-- 		</div> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div class="w-6 h-0.5" style="background-color: #8b5cf6"></div> -->
		<!-- 			<span class="text-xs">Job Connection</span> -->
		<!-- 		</div> -->
		<!-- 		<div class="flex items-center gap-1"> -->
		<!-- 			<div -->
		<!-- 				class="w-3 h-3 rounded-full border-2 border-white" -->
		<!-- 				style="background-color: {actionColors.Execute}" -->
		<!-- 			></div> -->
		<!-- 			<span class="text-xs">Clustered Points</span> -->
		<!-- 		</div> -->
		<!-- 	</div> -->
		<!-- </div> -->
	{/if}
</div>

<style>
	.timeline-container {
		max-height: 500px;
		overflow-y: auto;
	}
</style>
