<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Alert, Skeleton } from './common'
	import { Activity } from 'lucide-svelte'
	import { JobService } from '$lib/gen'
	import { msToReadableTime } from '$lib/utils'

	// OTEL SpanKind enum values from opentelemetry-proto/opentelemetry/proto/trace/v1/trace.proto
	const SpanKind = {
		UNSPECIFIED: 0,
		INTERNAL: 1,
		SERVER: 2,
		CLIENT: 3,
		PRODUCER: 4,
		CONSUMER: 5
	} as const

	// OTEL StatusCode enum values from opentelemetry-proto/opentelemetry/proto/trace/v1/trace.proto
	const StatusCode = {
		UNSET: 0,
		OK: 1,
		ERROR: 2
	} as const

	// Matches OTEL Span proto spec. If expanding usage, consider generating types from
	// opentelemetry-proto .proto files using ts-proto: npx protoc --ts_proto_out=./src/generated
	// See: https://github.com/open-telemetry/opentelemetry-proto
	interface OtelSpan {
		trace_id: string
		span_id: string
		parent_span_id: string | null
		name: string
		kind: number
		start_time_unix_nano: number
		end_time_unix_nano: number
		status: { code: number; message: string } | null
		attributes: Record<string, any>
	}

	interface Props {
		jobId: string
	}

	let { jobId }: Props = $props()
	let traces: OtelSpan[] = $state([])
	let loading = $state(true)
	let error: string | null = $state(null)
	let expandedSpans: Set<string> = $state(new Set())

	$effect(() => {
		if (jobId) {
			loadTraces()
		}
	})

	async function loadTraces() {
		if (!$workspaceStore || !jobId) return

		loading = true
		error = null

		try {
			const response = await JobService.getJobOtelTraces({
				workspace: $workspaceStore,
				id: jobId
			})
			traces = response as unknown as OtelSpan[]
		} catch (e: any) {
			if (e?.status === 404) {
				traces = []
			} else {
				error = `Error loading traces: ${e?.message ?? e}`
			}
		} finally {
			loading = false
		}
	}

	function getDurationMs(span: OtelSpan): number {
		return (span.end_time_unix_nano - span.start_time_unix_nano) / 1_000_000
	}

	function formatTimestamp(ns: number): string {
		const date = new Date(ns / 1000000)
		return date.toISOString()
	}

	function getStatusColor(statusCode: number): string {
		switch (statusCode) {
			case StatusCode.UNSET:
				return 'text-secondary'
			case StatusCode.OK:
				return 'text-green-600'
			case StatusCode.ERROR:
				return 'text-red-600'
			default:
				return 'text-secondary'
		}
	}

	function getStatusLabel(statusCode: number): string {
		switch (statusCode) {
			case StatusCode.UNSET:
				return 'Unset'
			case StatusCode.OK:
				return 'OK'
			case StatusCode.ERROR:
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

	function getKindLabel(kind: number): string {
		switch (kind) {
			case SpanKind.INTERNAL: return 'Internal'
			case SpanKind.SERVER: return 'Server'
			case SpanKind.CLIENT: return 'Client'
			case SpanKind.PRODUCER: return 'Producer'
			case SpanKind.CONSUMER: return 'Consumer'
			default: return 'Unknown'
		}
	}

	// Parse OTEL proto AnyValue to a simple value
	function parseAnyValue(anyValue: any): any {
		if (!anyValue) return null
		if (anyValue.stringValue !== undefined) return anyValue.stringValue
		if (anyValue.intValue !== undefined) return anyValue.intValue
		if (anyValue.boolValue !== undefined) return anyValue.boolValue
		if (anyValue.doubleValue !== undefined) return anyValue.doubleValue
		if (anyValue.arrayValue?.values) {
			return anyValue.arrayValue.values.map(parseAnyValue)
		}
		if (anyValue.kvlistValue?.values) {
			return parseAttributes(anyValue.kvlistValue.values)
		}
		return JSON.stringify(anyValue)
	}

	// Parse OTEL proto attributes array to a simple key-value object
	function parseAttributes(attributes: any): Record<string, any> {
		if (!attributes) return {}
		if (!Array.isArray(attributes)) return attributes // already parsed
		const result: Record<string, any> = {}
		for (const attr of attributes) {
			if (attr.key && attr.value !== undefined) {
				result[attr.key] = parseAnyValue(attr.value)
			}
		}
		return result
	}

	// Calculate timeline metrics
	function getTimelineMetrics(spans: OtelSpan[]) {
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
			<p class="text-lg font-medium">No HTTP requests captured</p>
			<p class="text-sm mt-2">
				This job did not make any HTTP/HTTPS requests, or HTTP Request Tracing is not enabled in instance settings.
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
						{@const durationNs = span.end_time_unix_nano - span.start_time_unix_nano}
						{@const startOffset =
							((span.start_time_unix_nano - timelineMetrics.minTime) /
								timelineMetrics.totalDuration) *
							100}
						{@const width = (durationNs / timelineMetrics.totalDuration) * 100}
						{@const statusCode = span.status?.code ?? 0}

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
										<span class="font-medium truncate" title={span.name}>
											{span.name}
										</span>
									</div>
									<div class="col-span-1">
										<span class={`text-xs font-medium ${getStatusColor(statusCode)}`}>
											{getStatusLabel(statusCode)}
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
										{msToReadableTime(getDurationMs(span))}
									</div>
								</div>
							</button>

							{#if expandedSpans.has(span.span_id)}
								{@const parsedAttrs = parseAttributes(span.attributes)}
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
										<div>
											<p class="text-xs text-secondary mb-1">Kind</p>
											<p class="font-mono text-xs">{getKindLabel(span.kind)}</p>
										</div>
										<div>
											<p class="text-xs text-secondary mb-1">Start Time</p>
											<p class="font-mono text-xs">{formatTimestamp(span.start_time_unix_nano)}</p>
										</div>
										<div>
											<p class="text-xs text-secondary mb-1">End Time</p>
											<p class="font-mono text-xs">{formatTimestamp(span.end_time_unix_nano)}</p>
										</div>
										{#if span.status?.message}
											<div class="col-span-2">
												<p class="text-xs text-secondary mb-1">Status Message</p>
												<p class="text-xs">{span.status.message}</p>
											</div>
										{/if}
										{#if Object.keys(parsedAttrs).length > 0}
											<div class="col-span-2">
												<p class="text-xs text-secondary mb-1">Attributes</p>
												<div class="bg-surface-secondary p-2 rounded text-xs space-y-1">
													{#each Object.entries(parsedAttrs) as [key, value]}
														<div class="flex gap-2">
															<span class="text-secondary font-medium shrink-0">{key}:</span>
															<span class="font-mono break-all">{typeof value === 'object' ? JSON.stringify(value) : value}</span>
														</div>
													{/each}
												</div>
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
