<script lang="ts">
	import { displayDate, msToSec } from '$lib/utils'
	import { getDbClockNow } from '$lib/forLater'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'
	import type { GlobalIterationBounds } from './graph'
	import FlowTimelineCompute from './FlowTimelineCompute.svelte'

	interface Props {
		selfWaitTime?: number | undefined
		aggregateWaitTime?: number | undefined
		flowModules: string[]
		durationStatuses: Record<
			string,
			{
				byJob: Record<string, { created_at?: number; started_at?: number; duration_ms?: number }>
			}
		>
		flowDone?: boolean
		decreaseIterationFrom?: (key: string, amount: number) => void
		buildSubflowKey: (key: string) => string
		globalIterationBounds: Record<string, GlobalIterationBounds>
	}

	let {
		selfWaitTime = undefined,
		aggregateWaitTime = undefined,
		flowModules,
		durationStatuses,
		flowDone = false,
		decreaseIterationFrom,
		buildSubflowKey,
		globalIterationBounds
	}: Props = $props()

	let min: undefined | number = $state(undefined)
	let max: undefined | number = $state(undefined)
	let total: number | undefined = $state(undefined)
	let items:
		| Record<
				string,
				Array<{ created_at?: number; started_at?: number; duration_ms?: number; id: string }>
		  >
		| undefined = $state(undefined)
	let now = $state(getDbClockNow().getTime())
	let flowTimelineCompute = $state<FlowTimelineCompute | undefined>(undefined)

	export function reset() {
		flowTimelineCompute?.reset()
	}
</script>

<FlowTimelineCompute
	{flowModules}
	{durationStatuses}
	{flowDone}
	bind:min
	bind:max
	bind:total
	bind:items
	bind:now
	bind:this={flowTimelineCompute}
/>

{#if items}
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
					<div>Waiting for executor/Suspend</div>
					<div class="h-4 w-4 bg-gray-500"></div>
				</div>

				<div class="flex gap-2 items-center">
					<div>Execution</div>
					<div class="h-4 w-4 bg-blue-500/90"></div>
				</div>
			</div>
		</div>
		{#if selfWaitTime}
			<div class="px-2 py-2 grid grid-cols-6 w-full">
				root:
				<WaitTimeWarning
					self_wait_time_ms={selfWaitTime}
					aggregate_wait_time_ms={aggregateWaitTime}
					variant="badge-self-wait"
				/>
			</div>
		{/if}
		{#each Object.values(flowModules) as k (k)}
			{@const iterationFrom = globalIterationBounds[buildSubflowKey(k)]?.iteration_from ?? 0}
			<div class="overflow-auto max-h-60 shadow-inner dark:shadow-gray-700 relative">
				{#if iterationFrom > 0}
					<div class="w-full flex flex-row-reverse sticky top-0">
						<button
							class="!text-secondary underline mr-2 text-2xs text-right whitespace-nowrap"
							onclick={() => {
								decreaseIterationFrom?.(k, 20)
							}}
							>Viewing iterations {iterationFrom} to {globalIterationBounds[buildSubflowKey(k)]
								?.iteration_total}. Load more
						</button>
					</div>
				{/if}

				<div class="px-2 py-2 grid grid-cols-6 w-full">
					<div class="truncate">{k.startsWith('subflow:') ? k.substring(8) : k}</div>
					<div class="col-span-5 flex min-h-6">
						{#if min && total}
							<div class="flex flex-col gap-2 w-full p-2 ml-4">
								{#each items?.[k] ?? [] as b}
									{@const waitingLen = b?.created_at
										? b.started_at
											? b.started_at - b?.created_at
											: b.duration_ms
												? 0
												: now - b?.created_at
										: 0}
									<div class="flex w-full">
										<TimelineBar
											position="left"
											id={b?.id}
											{total}
											{min}
											gray
											started_at={b.created_at}
											len={waitingLen < 100 ? 0 : waitingLen - 100}
											running={b?.started_at == undefined}
										/>
										{#if b.started_at}
											<TimelineBar
												position={waitingLen < 100 ? 'center' : 'right'}
												id={b?.id}
												{total}
												{min}
												concat
												started_at={b.started_at}
												len={b.started_at ? (b?.duration_ms ?? now - b?.started_at) : 0}
												running={b?.duration_ms == undefined}
											/>
										{/if}
									</div>
								{/each}
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
