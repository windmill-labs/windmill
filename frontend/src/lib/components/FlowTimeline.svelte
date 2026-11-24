<script lang="ts">
	import { displayDate, msToCompactDuration } from '$lib/utils'
	import { ConcurrencyGroupsService } from '$lib/gen'
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
		/** For a narrow panel: thin bars, with each step's duration beside its name. */
		compact?: boolean
	}

	let {
		selfWaitTime = undefined,
		aggregateWaitTime = undefined,
		flowModules,
		durationStatuses,
		flowDone = false,
		localModuleStates,
		onSelectedIteration,
		compact = false
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

	const barHeight = $derived(compact ? 10 : 32)

	// Whole approval-wait machinery below is inert unless a step actually has a suspend config,
	// so large suspend-free flows pay nothing for the flatten/sort and the per-tick recompute.
	const hasSuspendModule = $derived(flowModules.some((m) => m.suspend))

	// Push times of every job on the timeline, ascending. Used to locate when the step
	// that follows an approval step started — i.e. the moment the approval was granted.
	const allCreatedAts = $derived(
		hasSuspendModule
			? Object.values(items ?? {})
					.flat()
					.map((j) => j.created_at)
					.filter((t): t is number => t != undefined)
					.sort((a, b) => a - b)
			: []
	)

	// Heuristic: the grant moment is approximated by the next job pushed anywhere on the
	// timeline. Exact for sequential flows; for an approval step inside one branch of a
	// parallel branchall a concurrent sibling job can land first and understate the wait.
	function nextCreatedAtAfter(t: number): number | undefined {
		return allCreatedAts.find((c) => c > t)
	}

	// For a completed suspend/approval step, the time spent waiting for the approval is the
	// gap between the step finishing and the next step being pushed (or now, if still waiting).
	function approvalWait(b: {
		started_at?: number
		duration_ms?: number
	}): { start: number; len: number; running: boolean } | undefined {
		if (b.started_at == undefined || b.duration_ms == undefined) {
			return undefined
		}
		const end = b.started_at + b.duration_ms
		const next = nextCreatedAtAfter(end)
		const waitEnd = next ?? (flowDone ? undefined : now)
		if (waitEnd == undefined) {
			return undefined
		}
		const len = waitEnd - end
		if (len < 100) {
			return undefined
		}
		return { start: end, len, running: next == undefined }
	}

	// Approval wait per module id, computed once and consumed by both the rows and the legend.
	const approvalWaitByModule = $derived.by(() => {
		const result: Record<string, { start: number; len: number; running: boolean }> = {}
		for (const m of flowModules) {
			if (!m.suspend) continue
			const sub = (items?.[m.id] ?? []).filter((x) => x.created_at && x.started_at)
			if (sub.length !== 1) continue
			const aw = approvalWait(sub[0])
			if (aw) result[m.id] = aw
		}
		return result
	})

	const hasApprovalWait = $derived(Object.keys(approvalWaitByModule).length > 0)

	// The hold of a limited step's job once it started; a live hold comes from the step's state.
	let holds = $state<Record<string, { from?: number; to?: number } | null>>({})
	$effect(() => {
		for (const m of flowModules) {
			if (!m.limited) continue
			for (const j of items?.[m.id] ?? []) {
				if (!j.started_at || j.id in untrack(() => holds)) continue
				holds[j.id] = null
				ConcurrencyGroupsService.getConcurrencyKeyStatus({ id: j.id })
					.then((s) => {
						if (s?.deferred_from)
							holds[j.id] = {
								from: new Date(s.deferred_from).getTime(),
								to: s.released_at ? new Date(s.released_at).getTime() : undefined
							}
					})
					.catch(() => {})
			}
		}
	})

	type WaitSegment = { start: number; len: number; kind: 'sleep' | 'held' | 'wait' }
	/** A job's wait split into the sleep before it, its concurrency hold and the rest. */
	function waitSegments(
		m: FlowModuleForTimeline,
		job: { id: string; created_at?: number; started_at?: number },
		single: boolean
	): WaitSegment[] {
		const b = { ...job, created_at: job.created_at ?? now }
		const end = b.started_at ?? now
		const state = localModuleStates[m.id]
		const live = b.started_at == undefined && state?.type === 'WaitingForExecutor'
		let sleepEnd: number | undefined
		if (single && live && state?.waiting_reason === 'sleep' && state.scheduled_for)
			sleepEnd = new Date(state.scheduled_for).getTime()
		else if (single && m.sleepBefore) sleepEnd = b.created_at + m.sleepBefore * 1000
		const hold =
			live && state?.waiting_reason === 'concurrency' && state.held_since
				? { from: new Date(state.held_since).getTime(), to: undefined }
				: holds[b.id]
		const clamp = (t: number) => Math.min(Math.max(t, b.created_at), end)
		const segments: WaitSegment[] = []
		let at = b.created_at
		const push = (to: number, kind: WaitSegment['kind']) => {
			to = clamp(to)
			if (to > at) segments.push({ start: at, len: to - at, kind })
			at = Math.max(at, to)
		}
		if (sleepEnd != undefined) push(sleepEnd, 'sleep')
		if (hold?.from != undefined) {
			push(hold.from, 'wait')
			push(hold.to ?? end, 'held')
		}
		push(end, 'wait')
		return segments
	}
	const segmentStyle = {
		// A wait like the gray one, striped as one the flow asked for.
		sleep: {
			colorClass:
				'bg-[repeating-linear-gradient(135deg,#d1d5db_0_4px,#e5e7eb_4px_8px)] dark:bg-[repeating-linear-gradient(135deg,#4b5563_0_4px,#374151_4px_8px)] !text-gray-700 dark:!text-gray-100',
			label: 'Sleep'
		},
		held: { colorClass: 'bg-orange-400/90', label: 'Held by concurrency limit' },
		wait: { colorClass: undefined, label: 'Wait' }
	}
	const hasSleep = $derived(
		flowModules.some((m) => m.sleepBefore) ||
			Object.values(localModuleStates).some((s) => s.waiting_reason === 'sleep')
	)
	const hasHold = $derived(
		Object.values(holds).some((h) => h?.from != undefined) ||
			Object.values(localModuleStates).some((s) => s.waiting_reason === 'concurrency')
	)
</script>

<OnChange
	key={durationStatuses}
	onChange={() => {
		timelineCompute?.updateInputs(flowModulesIds, durationStatuses, flowDone)
	}}
/>
{#if items}
	<div class={compact ? 'flex flex-col gap-3 py-2' : 'divide-y'}>
		<div class="px-3 py-1.5 flex items-center justify-between text-2xs text-secondary">
			{#if !compact}
				<div class="flex gap-1 items-center font-mono">
					{min ? displayDate(new Date(min), true) : ''}
				</div>
			{/if}
			<div class="flex gap-3 items-center {compact ? 'w-full [&>span:last-child]:ml-auto' : ''}">
				<div class="flex gap-1.5 items-center">
					<div class="h-2.5 w-2.5 rounded-sm bg-gray-400 dark:bg-gray-500"></div>
					<span>Wait</span>
				</div>
				<div class="flex gap-1.5 items-center">
					<div class="h-2.5 w-2.5 rounded-sm bg-blue-500/90"></div>
					<span>Execution</span>
				</div>
				{#if hasSleep}
					<div class="flex gap-1.5 items-center">
						<div
							class="h-2.5 w-2.5 rounded-sm bg-[repeating-linear-gradient(135deg,#d1d5db_0_4px,#e5e7eb_4px_8px)] dark:bg-[repeating-linear-gradient(135deg,#4b5563_0_4px,#374151_4px_8px)]"
						></div>
						<span>Sleep</span>
					</div>
				{/if}
				{#if hasHold}
					<div class="flex gap-1.5 items-center">
						<div class="h-2.5 w-2.5 rounded-sm bg-orange-400/90"></div>
						<span>Concurrency limit</span>
					</div>
				{/if}
				{#if hasApprovalWait}
					<div class="flex gap-1.5 items-center">
						<div class="h-2.5 w-2.5 rounded-sm bg-purple-400/80"></div>
						<span>Approval wait</span>
					</div>
				{/if}
				{#if max && min}
					<span class="font-mono">{msToCompactDuration(max - min, 1)}</span>
				{/if}
				{#if !max && min}{#if now}
						<span class="font-mono">{msToCompactDuration(now - min, 1)}</span>
					{/if}<Loader2 size={14} class="animate-spin" />{/if}
			</div>
		</div>
		{#if selfWaitTime}
			<div class="px-3 py-1.5 flex items-center gap-2">
				<span class="text-xs text-secondary">root:</span>
				<WaitTimeWarning
					self_wait_time_ms={selfWaitTime}
					aggregate_wait_time_ms={aggregateWaitTime}
					variant="badge-self-wait"
				/>
			</div>
		{/if}
		{#each flowModules as m (m.id)}
			{@const { id: k, type: typ, suspend: isSuspend, label } = m}
			{@const subItems = items?.[k]?.filter(
				(x) => x.created_at && (x.started_at || localModuleStates[k]?.waiting_reason)
			)}
			<div class="relative px-3 py-1.5">
				<div class="flex items-center justify-between mb-0.5">
					<div class="text-xs font-medium"
						>{label ?? (k.startsWith('subflow:') ? k.substring(8) : k)}
						{#if localModuleStates[k]?.selectedForloop && (typ == 'forloopflow' || typ == 'whileloopflow')}
							<span class="text-xs font-mono font-medium inline-flex items-center -my-2">
								<button onclick={(e) => e.stopPropagation()}>
									<FlowJobsMenu
										moduleId={k}
										id={k}
										{onSelectedIteration}
										flowJobsSuccess={localModuleStates[k]?.flow_jobs_success}
										flowJobsLabels={localModuleStates[k]?.flow_jobs_labels}
										flowJobs={localModuleStates[k]?.flow_jobs}
										selected={localModuleStates[k]?.selectedForloopIndex ?? 0}
										selectedManually={localModuleStates[k]?.selectedForLoopSetManually ?? false}
										showIcon={false}
									/>
								</button>
							</span>
						{/if}
					</div>
					{#if subItems?.length > 1}
						<span class="text-2xs text-secondary bg-surface-hover px-1.5 py-0.5 rounded-full">
							{subItems?.length} jobs
						</span>
					{:else if compact && subItems?.[0]?.duration_ms != undefined}
						<span class="text-2xs text-secondary font-mono">
							{msToCompactDuration(subItems[0].duration_ms, 1)}
						</span>
					{/if}
				</div>
				<div class="w-full">
					{#if min && total}
						<VirtualList
							width="100%"
							height={Math.min(400, (subItems?.length ?? 0) * barHeight)}
							itemCount={subItems?.length ?? 0}
							itemSize={barHeight}
							getKey={(index) => subItems?.[index]?.id ?? `_${index}`}
						>
							{#snippet item({ index, style })}
								{@const b = subItems?.[index]}
								{#if b?.created_at}
									{@const waitingLen = b?.created_at
										? b.started_at
											? b.started_at - b?.created_at
											: b.duration_ms
												? 0
												: now - b?.created_at
										: 0}
									{@const aw = isSuspend ? approvalWaitByModule[k] : undefined}
									{@const segments =
										waitingLen < 100 ? [] : waitSegments(m, b, subItems?.length === 1)}
									<div class="flex w-full py-0.5 items-center" {style}>
										{#if segments.length <= 1}
											<TimelineBar
												position="left"
												id={b?.id}
												{total}
												{min}
												{compact}
												gray
												colorClass={segmentStyle[segments[0]?.kind ?? 'wait'].colorClass}
												tooltip={segments[0] && segments[0].kind !== 'wait'
													? `${segmentStyle[segments[0].kind].label} · ${msToCompactDuration(segments[0].len, 1)}`
													: undefined}
												followedByBar={!!b.started_at}
												spacerClass="bg-gray-100 dark:bg-gray-800/50 rounded-l-md"
												started_at={b.created_at}
												len={waitingLen < 100 ? 0 : waitingLen - 100}
												running={b?.started_at == undefined}
											/>
										{:else}
											{#each segments as seg, i (i)}
												<TimelineBar
													position={i === 0 ? 'left' : 'none'}
													id={b?.id}
													{total}
													{min}
													{compact}
													gray
													concat={i > 0}
													colorClass={segmentStyle[seg.kind].colorClass}
													tooltip="{segmentStyle[seg.kind].label} · {msToCompactDuration(
														seg.len,
														1
													)}"
													followedByBar
													spacerClass="bg-gray-100 dark:bg-gray-800/50 rounded-l-md"
													started_at={seg.start}
													len={i === segments.length - 1 ? Math.max(seg.len - 100, 0) : seg.len}
													running={b?.started_at == undefined}
												/>
											{/each}
										{/if}
										{#if b.started_at}
											<TimelineBar
												position={aw || waitingLen < 100 ? 'center' : 'right'}
												id={b?.id}
												{total}
												{min}
												{compact}
												concat
												started_at={b.started_at}
												len={b.started_at ? (b?.duration_ms ?? now - b?.started_at) : 0}
												running={b?.duration_ms == undefined}
											/>
										{/if}
										{#if aw}
											<TimelineBar
												position="right"
												id={b?.id}
												{total}
												{min}
												{compact}
												concat
												colorClass="bg-purple-400/80"
												tooltip={`Waiting for approval — ${msToCompactDuration(aw.len, 1)}`}
												started_at={aw.start}
												len={aw.len}
												running={aw.running}
											/>
										{/if}
									</div>
								{:else}
									<div class="flex w-full py-0.5"></div>
								{/if}
							{/snippet}
						</VirtualList>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin" />
{/if}
