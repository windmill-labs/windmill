<script lang="ts">
	import { base } from '$lib/base'
	import { displayDate, msToSec } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import { getDbClockNow } from '$lib/forLater'
	import { ExternalLink, Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import type { WorkflowStatus } from '$lib/gen'

	export let flow_status: Record<string, WorkflowStatus>
	export let flowDone = false

	$: min = Object.values(flow_status).reduce(
		(a, b) => Math.min(a, b.scheduled_for ? new Date(b.scheduled_for).getTime() : Infinity),
		Infinity
	)
	$: max = flowDone
		? Object.values(flow_status).reduce(
				(a, b) =>
					Math.max(a, b.started_at ? new Date(b.started_at).getTime() + (b.duration_ms ?? 0) : 0),
				0
		  )
		: undefined
	$: total = flowDone && max ? max - min : now - min

	let now = getDbClockNow().getTime()

	let interval = setInterval((x) => {
		if (!max) {
			now = getDbClockNow().getTime()
		}
		if (min && (!max || total == undefined)) {
			total = max ? max - min : Math.max(now - min, 2000)
		}
	}, 30)

	onDestroy(() => {
		interval && clearInterval(interval)
	})
</script>

{#if flow_status}
	<div class="divide-y border-b">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div></div>
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between"
				><div>{min ? displayDate(new Date(min), true) : ''}</div>{#if max && min}<div
						class="hidden lg:block">{msToSec(max - min)}s</div
					>
				{/if}<div class="flex gap-1 items-center font-mono"
					>{max ? displayDate(new Date(max), true) : ''}{#if !max && min}{#if now}
							{msToSec(now - min, 3)}s
						{/if}<Loader2 size={14} class="animate-spin" />{/if}</div
				></div
			>
		</div>
		<div class="flex flex-row-reverse items-center text-sm text-secondary p-2">
			<div class="flex gap-4 items-center text-2xs">
				<div class="flex gap-2 items-center">
					<div>Waiting for executor</div>
					<div class="h-4 w-4 bg-gray-500"></div>
				</div>

				<div class="flex gap-2 items-center">
					<div>Execution</div>
					<div class="h-4 w-4 bg-blue-500/90"></div>
				</div>
			</div>
		</div>
		{#each Object.entries(flow_status) as [k, v] (k)}
			<div class="overflow-auto max-h-60 shadow-inner dark:shadow-gray-700 relative">
				<div class="px-2 py-2 text-xs grid grid-cols-6 w-full gap-1">
					<a target="_blank" class="inline-flex gap-2 items-baseline" href="{base}/run/{k}"
						>{v.name ?? k} <ExternalLink size={12} /></a
					>
					<div class="col-span-5 flex min-h-6 w-full">
						{#if min && total}
							{@const scheduledFor = v?.scheduled_for
								? new Date(v?.scheduled_for).getTime()
								: undefined}
							{@const startedAt = v?.started_at ? new Date(v?.started_at).getTime() : undefined}

							{@const waitingLen = scheduledFor
								? startedAt
									? startedAt - scheduledFor
									: now - scheduledFor
								: 0}

							<div class="flex w-full">
								<TimelineBar
									position="left"
									id={k}
									{total}
									{min}
									gray
									started_at={scheduledFor}
									len={waitingLen < 100 ? 0 : waitingLen - 100}
									running={startedAt == undefined}
								/>
								{#if startedAt}
									<TimelineBar
										position={waitingLen < 100 ? 'center' : 'right'}
										id={k}
										{total}
										{min}
										concat
										started_at={startedAt}
										len={v.duration_ms ?? now - startedAt}
										running={v.duration_ms == undefined}
									/>
								{/if}
							</div>
						{/if}</div
					></div
				>
			</div>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
