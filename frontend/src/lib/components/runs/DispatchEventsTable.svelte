<script lang="ts">
	import { base } from '$lib/base'
	import type { ListDispatchEventsResponse } from '$lib/gen'
	import { ExternalLink, Check, Hourglass, MinusCircle } from 'lucide-svelte'

	type Event = ListDispatchEventsResponse[number]
	type Props = { events: Event[]; workspace: string }
	let { events, workspace }: Props = $props()

	// These reason strings are produced verbatim by the backend asset-dispatch
	// reason enum (backend asset_dispatch.rs) — keep them in sync if that enum
	// changes; an unmatched reason falls through to the raw string.
	function reasonLabel(reason: string | undefined): string {
		switch (reason) {
			case 'self_loop':
				return 'subscriber path equals producer path'
			case 'case3_non_partition_bearing':
				return 'reference input — only {partition}-bearing edges advance the join'
			case 'case3_missing_partition':
				return 'producer ran without a resolved partition'
			case 'cycle_detected':
				return 'subscriber already ran upstream in this chain (cycle)'
			default:
				return reason ?? ''
		}
	}
</script>

<table class="w-full text-xs">
	<thead class="bg-surface-secondary text-secondary">
		<tr>
			<th class="text-left font-normal px-3 py-2">Outcome</th>
			<th class="text-left font-normal px-3 py-2">Subscriber</th>
			<th class="text-left font-normal px-3 py-2">Asset</th>
			<th class="text-left font-normal px-3 py-2">Detail</th>
		</tr>
	</thead>
	<tbody>
		{#each events as ev (ev.created_at + ev.subscriber_path + ev.asset_path)}
			<tr class="border-t">
				<td class="px-3 py-2 align-top whitespace-nowrap">
					{#if ev.outcome === 'dispatched'}
						<span
							class="inline-flex items-center text-green-700 dark:text-green-400"
							title="dispatched"
						>
							<Check size={12} />
						</span>
					{:else if ev.outcome === 'join_pending'}
						<span class="inline-flex items-center gap-1 text-yellow-700 dark:text-yellow-400">
							<Hourglass size={12} /> join pending
						</span>
					{:else}
						<span class="inline-flex items-center gap-1 text-secondary">
							<MinusCircle size={12} /> skipped
						</span>
					{/if}
				</td>
				<td class="px-3 py-2 align-top">
					<span class="font-mono">{ev.subscriber_path}</span>
				</td>
				<td class="px-3 py-2 align-top">
					<span class="font-mono text-secondary">{ev.asset_kind}://{ev.asset_path}</span>
				</td>
				<td class="px-3 py-2 align-top">
					{#if ev.outcome === 'dispatched' && ev.child_job_id}
						<a
							class="inline-flex items-center gap-1"
							href={`${base}/run/${ev.child_job_id}?workspace=${workspace}`}
							target="_blank"
							rel="noopener noreferrer"
						>
							<span class="font-mono">{ev.child_job_id.slice(0, 8)}…</span>
							<ExternalLink size={12} />
						</a>
						{#if ev.partition}
							<span class="text-secondary">
								· partition <span class="font-mono">{ev.partition}</span></span
							>
						{/if}
						{#if ev.debounce_s && ev.debounce_s > 0}
							<span class="text-secondary"> · debounced {ev.debounce_s}s</span>
						{/if}
					{:else if ev.outcome === 'join_pending'}
						<span>{ev.received_inputs ?? 0}/{ev.required_inputs ?? 0} inputs received</span>
						{#if ev.partition}
							<span class="text-secondary">
								for partition <span class="font-mono">{ev.partition}</span></span
							>
						{/if}
					{:else}
						<span class="text-secondary">{reasonLabel(ev.reason)}</span>
					{/if}
				</td>
			</tr>
		{/each}
	</tbody>
</table>
