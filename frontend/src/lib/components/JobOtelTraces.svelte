<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { onMount } from 'svelte'
	import { Alert, Skeleton } from './common'
	import { Activity } from 'lucide-svelte'

	interface OtelTraceSpan {
		trace_id: string
		span_id: string
		parent_span_id: string | null
		operation_name: string
		service_name: string | null
		start_time_unix_nano: number
		end_time_unix_nano: number
		duration_ns: number
		status_code: number
		status_message: string | null
		attributes: Record<string, any>
		events: any[]
	}

	interface Props {
		jobId: string
	}

	let { jobId }: Props = $props()
	let traces: OtelTraceSpan[] = $state([])
	let loading = $state(true)
	let error: string | null = $state(null)
	let expandedSpans: Set<string> = $state(new Set())

	onMount(async () => {
		await loadTraces()
	})

	async function loadTraces() {
		if (!$workspaceStore || !jobId) return

		loading = true
		error = null

		try {
			const response = await fetch(
				`/api/w/${$workspaceStore}/jobs/get_otel_traces/${jobId}`
			)
			if (response.ok) {
				traces = await response.json()
			} else if (response.status === 404) {
				traces = []
			} else {
				error = `Failed to load traces: ${response.statusText}`
			}
		} catch (e) {
			error = `Error loading traces: ${e}`
		} finally {
			loading = false
		}
	}

	function formatDuration(ns: number): string {
		if (ns < 1000) return `${ns}ns`
		if (ns < 1000000) return `${(ns / 1000).toFixed(2)}μs`
		if (ns < 1000000000) return `${(ns / 1000000).toFixed(2)}ms`
		return `${(ns / 1000000000).toFixed(2)}s`
	}

	function formatTimestamp(ns: number): string {
		const date = new Date(ns / 1000000)
		return date.toISOString()
	}

	function getStatusColor(statusCode: number): string {
		switch (statusCode) {
			case 0: // Unset
				return 'text-secondary'
			case 1: // Ok
				return 'text-green-600'
			case 2: // Error
				return 'text-red-600'
			default:
				return 'text-secondary'
		}
	}

	function getStatusLabel(statusCode: number): string {
		switch (statusCode) {
			case 0:
				return 'Unset'
			case 1:
				return 'OK'
			case 2:
				return 'Error'
			default:
				return 'Unknown'
		}
	}

	function toggleSpan(spanId: string) {
		const newSet = new Set(expandedSpans)
		if (newSet.has(spanId)) {
			newSet.delete(spanId)
		} else {
			newSet.add(spanId)
		}
		expandedSpans = newSet
	}

	// Calculate timeline metrics
	function getTimelineMetrics(spans: OtelTraceSpan[]) {
		if (spans.length === 0) return { minTime: 0, maxTime: 0, totalDuration: 0 }
		const minTime = Math.min(...spans.map((s) => s.start_time_unix_nano))
		const maxTime = Math.max(...spans.map((s) => s.end_time_unix_nano))
		return { minTime, maxTime, totalDuration: maxTime - minTime }
	}

	let timelineMetrics = $derived(getTimelineMetrics(traces))
</script>

<div class="p-4">
	{#if loading}
		<Skeleton layout={[[4], [8], [6], [10]]} />
	{:else if error}
		<Alert type="error" title="Error">{error}</Alert>
	{:else if traces.length === 0}
		<div class="flex flex-col items-center justify-center py-8 text-secondary">
			<Activity size={48} class="mb-4 opacity-50" />
			<p class="text-lg font-medium">No traces found</p>
			<p class="text-sm mt-2">
				This job did not generate any OTel traces. Make sure OTel auto-instrumentation is enabled
				and the script uses OTel libraries.
			</p>
		</div>
	{:else}
		<div class="space-y-4">
			<div class="flex items-center justify-between">
				<h3 class="text-lg font-semibold">Traces ({traces.length} spans)</h3>
				<button
					class="text-sm text-blue-600 hover:underline"
					onclick={loadTraces}
				>
					Refresh
				</button>
			</div>

			<!-- Timeline view -->
			<div class="border rounded-lg overflow-hidden">
				<div class="bg-surface-secondary px-4 py-2 border-b">
					<div class="grid grid-cols-12 gap-2 text-xs font-medium text-secondary">
						<div class="col-span-4">Operation</div>
						<div class="col-span-1">Status</div>
						<div class="col-span-5">Timeline</div>
						<div class="col-span-2 text-right">Duration</div>
					</div>
				</div>

				<div class="divide-y">
					{#each traces as span (span.span_id)}
						{@const startOffset =
							((span.start_time_unix_nano - timelineMetrics.minTime) /
								timelineMetrics.totalDuration) *
							100}
						{@const width = (span.duration_ns / timelineMetrics.totalDuration) * 100}

						<div class="hover:bg-surface-hover">
							<button
								class="w-full px-4 py-2 text-left"
								onclick={() => toggleSpan(span.span_id)}
							>
								<div class="grid grid-cols-12 gap-2 items-center">
									<div class="col-span-4 flex items-center gap-2">
										<span class="text-xs text-secondary">
											{expandedSpans.has(span.span_id) ? '▼' : '▶'}
										</span>
										<span class="font-medium truncate" title={span.operation_name}>
											{span.operation_name}
										</span>
									</div>
									<div class="col-span-1">
										<span class={`text-xs font-medium ${getStatusColor(span.status_code)}`}>
											{getStatusLabel(span.status_code)}
										</span>
									</div>
									<div class="col-span-5 relative h-4">
										<div class="absolute inset-0 bg-surface-secondary rounded"></div>
										<div
											class="absolute h-full bg-blue-500 rounded opacity-75"
											style="left: {startOffset}%; width: {Math.max(width, 0.5)}%;"
										></div>
									</div>
									<div class="col-span-2 text-right text-sm font-mono">
										{formatDuration(span.duration_ns)}
									</div>
								</div>
							</button>

							{#if expandedSpans.has(span.span_id)}
								<div class="px-4 pb-4 bg-surface-secondary/50">
									<div class="grid grid-cols-2 gap-4 text-sm">
										<div>
											<p class="text-xs text-secondary mb-1">Trace ID</p>
											<p class="font-mono text-xs break-all">{span.trace_id}</p>
										</div>
										<div>
											<p class="text-xs text-secondary mb-1">Span ID</p>
											<p class="font-mono text-xs">{span.span_id}</p>
										</div>
										{#if span.parent_span_id}
											<div>
												<p class="text-xs text-secondary mb-1">Parent Span ID</p>
												<p class="font-mono text-xs">{span.parent_span_id}</p>
											</div>
										{/if}
										{#if span.service_name}
											<div>
												<p class="text-xs text-secondary mb-1">Service</p>
												<p class="font-mono text-xs">{span.service_name}</p>
											</div>
										{/if}
										<div>
											<p class="text-xs text-secondary mb-1">Start Time</p>
											<p class="font-mono text-xs">{formatTimestamp(span.start_time_unix_nano)}</p>
										</div>
										<div>
											<p class="text-xs text-secondary mb-1">End Time</p>
											<p class="font-mono text-xs">{formatTimestamp(span.end_time_unix_nano)}</p>
										</div>
										{#if span.status_message}
											<div class="col-span-2">
												<p class="text-xs text-secondary mb-1">Status Message</p>
												<p class="text-xs">{span.status_message}</p>
											</div>
										{/if}
										{#if Object.keys(span.attributes).length > 0}
											<div class="col-span-2">
												<p class="text-xs text-secondary mb-1">Attributes</p>
												<pre class="text-xs bg-surface-secondary p-2 rounded overflow-x-auto">{JSON.stringify(span.attributes, null, 2)}</pre>
											</div>
										{/if}
										{#if span.events && span.events.length > 0}
											<div class="col-span-2">
												<p class="text-xs text-secondary mb-1">Events ({span.events.length})</p>
												<pre class="text-xs bg-surface-secondary p-2 rounded overflow-x-auto">{JSON.stringify(span.events, null, 2)}</pre>
											</div>
										{/if}
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		</div>
	{/if}
</div>
