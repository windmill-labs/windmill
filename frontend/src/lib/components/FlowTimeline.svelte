<script lang="ts">
	import { displayDate, msToSec } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'
	import { TimelineCompute } from '$lib/timelineCompute.svelte'
	import { onMount } from 'svelte'
	import OnChange from './common/OnChange.svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import { Tween } from 'svelte/motion'

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
		buildSubflowKey: (key: string) => string
	}

	let {
		selfWaitTime = undefined,
		aggregateWaitTime = undefined,
		flowModules,
		durationStatuses,
		flowDone = false
	}: Props = $props()

	let timelineCompute = $state<TimelineCompute | undefined>(undefined)

	// Initialize timeline compute when we have duration statuses
	onMount(() => {
		timelineCompute = new TimelineCompute(flowModules, durationStatuses, flowDone)
		return () => {
			timelineCompute?.destroy()
		}
	})

	// Derived timeline values
	const min = $derived(timelineCompute?.min ?? undefined)
	const max = $derived(timelineCompute?.max ?? undefined)
	const total = $derived(timelineCompute?.total ?? undefined)
	const items = $derived(timelineCompute?.items ?? undefined)
	const now = $derived(timelineCompute?.now ?? Date.now())
	const duration = new Tween(0, { duration: 1000 })

	$effect(() => {
		duration.set(max && min ? max - min : 0)
	})

	export function reset() {
		timelineCompute?.reset()
	}

	const barHeight = 20
</script>

<OnChange
	key={durationStatuses}
	onChange={() => {
		timelineCompute?.updateInputs(flowModules, durationStatuses, flowDone)
	}}
/>

{#if items}
	<div class="divide-y border-b">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div></div>
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between"
				><div>{min ? displayDate(new Date(min), true) : ''}</div>{#if max && min}<div
						class="hidden lg:block">{msToSec(duration.current, 1)}s</div
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
			<div class="shadow-inner dark:shadow-gray-700 relative">
				<div class="px-2 py-2 grid grid-cols-6 w-full">
					<div class="truncate">{k.startsWith('subflow:') ? k.substring(8) : k}</div>
					<div class="col-span-5 flex">
						{#if min && total}
							<VirtualList
								width="100%"
								height={Math.min(400, (items?.[k]?.length ?? 0) * barHeight)}
								itemCount={items?.[k]?.length ?? 0}
								itemSize={barHeight}
							>
								{#snippet item({ index, style })}
									{@const b = items?.[k]?.[index]}
									{@const waitingLen = b?.created_at
										? b.started_at
											? b.started_at - b?.created_at
											: b.duration_ms
												? 0
												: now - b?.created_at
										: 0}
									<div class="flex w-full p-1 pb-2 pl-12" {style}>
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
								{/snippet}
							</VirtualList>
						{/if}</div
					></div
				>
			</div>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
