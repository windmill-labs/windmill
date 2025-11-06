<script lang="ts">
	import { displayDate, msToSec } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import TimelineBar from './TimelineBar.svelte'
	import WaitTimeWarning from './common/waitTimeWarning/WaitTimeWarning.svelte'
	import { TimelineCompute } from '$lib/timelineCompute.svelte'
	import { onMount, untrack } from 'svelte'
	import OnChange from './common/OnChange.svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import type { GraphModuleState } from './graph'
	import FlowJobsMenu from './flows/map/FlowJobsMenu.svelte'
	import type { FlowModuleForTimeline } from './FlowStatusViewerInner.svelte'

	interface Props {
		selfWaitTime?: number | undefined
		aggregateWaitTime?: number | undefined
		flowModules: FlowModuleForTimeline[]
		localModuleStates: Record<string, GraphModuleState>

		onSelectedIteration?: (
			detail:
				| { id: string; index: number; manuallySet: true; moduleId: string }
				| { manuallySet: false; moduleId: string }
		) => Promise<void>
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
		flowDone = false,
		localModuleStates,
		onSelectedIteration
	}: Props = $props()

	let timelineCompute = $state<TimelineCompute | undefined>(undefined)
	const flowModulesIds = $derived(flowModules.map(({ id }) => id))

	// Initialize timeline compute when we have duration statuses
	onMount(() => {
		timelineCompute = new TimelineCompute(flowModulesIds, durationStatuses, flowDone)
		return () => {
			timelineCompute?.destroy()
		}
	})

	$effect(() => {
		flowDone
		untrack(() => {
			timelineCompute?.setFlowDone(flowDone)
		})
	})

	// Derived timeline values
	const min = $derived(timelineCompute?.min ?? undefined)
	const max = $derived(timelineCompute?.max ?? undefined)
	const total = $derived(timelineCompute?.total ?? undefined)
	const items = $derived(timelineCompute?.items ?? undefined)
	const now = $derived(timelineCompute?.now ?? Date.now())

	export function reset() {
		timelineCompute?.reset()
	}

	const barHeight = 32
</script>

<OnChange
	key={durationStatuses}
	onChange={() => {
		timelineCompute?.updateInputs(flowModulesIds, durationStatuses, flowDone)
	}}
/>
{#if items}
	<div class="divide-y border-b">
		<div class="px-2 py-2 grid grid-cols-12 w-full"
			><div></div>
			<div class="col-span-11 pt-1 px-2 flex text-2xs text-secondary justify-between"
				><div>{min ? displayDate(new Date(min), true) : ''}</div>{#if max && min}<div
						class="hidden lg:block">{msToSec(max - min, 1)}s</div
					>
				{/if}<div class="flex gap-1 items-center font-mono"
					>{max ? displayDate(new Date(max), true) : ''}{#if !max && min}{#if now}
							{msToSec(now - min, 1)}s
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
		{#each flowModules as { id: k, type: typ } (k)}
			{@const subItems = items?.[k]?.filter((x) => x.created_at && x.started_at)}
			<div class="shadow-inner dark:shadow-gray-700 relative">
				<div class="px-2 py-2 grid grid-cols-6 w-full">
					<div class="truncate"
						>{k.startsWith('subflow:') ? k.substring(8) : k}
						{#if localModuleStates[k]?.selectedForloop && (typ == 'forloopflow' || typ == 'whileloopflow')}
							<span class="text-xs font-mono font-medium inline-flex items-center -my-2">
								<button onclick={(e) => e.stopPropagation()}>
									<FlowJobsMenu
										moduleId={k}
										id={k}
										{onSelectedIteration}
										flowJobsSuccess={localModuleStates[k]?.flow_jobs_success}
										flowJobs={localModuleStates[k]?.flow_jobs}
										selected={localModuleStates[k]?.selectedForloopIndex ?? 0}
										selectedManually={localModuleStates[k]?.selectedForLoopSetManually ?? false}
										showIcon={false}
									/>
								</button>
							</span>
						{/if}
					</div>
					<div class="col-span-5 flex">
						{#if subItems?.length > 1}
							<div class="text-xs text-secondary absolute top-1 right-2">
								{subItems?.length} jobs
							</div>
						{/if}
						{#if min && total}
							<VirtualList
								width="100%"
								height={Math.min(400, (subItems?.length ?? 0) * barHeight)}
								itemCount={subItems?.length ?? 0}
								itemSize={barHeight}
								getKey={(index) => subItems?.[index]?.id}
							>
								{#snippet item({ index, style })}
									{@const b = subItems?.[index]}
									{#if b?.created_at}
										<!-- <div class="text-xs text-secondary">{JSON.stringify(b)}</div> -->
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
									{:else}
										<div class="flex w-full p-1 pb-2 pl-12">
											<div class="text-xs text-secondary">
												<!-- Waiting for executor/Suspend {JSON.stringify(b)} -->
											</div>
										</div>
									{/if}
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
