<script lang="ts">
	import type { Job } from '$lib/gen'
	import { setActiveReplay } from './flowRecording.svelte'
	import type { PipelineRecording, RecordedJob, RecordedNodeState } from './types'
	import AssetGraphCanvas from '$lib/components/assets/AssetGraph/AssetGraphCanvas.svelte'
	import type { AssetGraphSelection } from '$lib/components/assets/AssetGraph/types'
	import type { RunnableRunState } from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { CheckCircle2, InfoIcon, Loader2, LogOut, Play, Square, XCircle } from 'lucide-svelte'
	import { onDestroy, tick, untrack } from 'svelte'

	type ReplayState = 'loaded' | 'playing' | 'done'

	interface Props {
		recording: PipelineRecording
		hideControls?: boolean
	}

	let { recording, hideControls = false }: Props = $props()

	let replayState = $state<ReplayState>('loaded')
	let currentStatuses: Record<string, RecordedNodeState> = $state({})
	let selection: AssetGraphSelection | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)
	let jobLoader: JobLoader | undefined = $state(undefined)
	let timeouts: ReturnType<typeof setTimeout>[] = []

	let jobDone = $derived((job as any)?.type === 'CompletedJob')

	// last-wins map path -> job id across all recorded frames, so any node that
	// ran is clickable regardless of the current animation position.
	let pathToJobId = $derived.by(() => {
		const m: Record<string, string> = {}
		for (const frame of recording.timeline) {
			for (const [path, st] of Object.entries(frame.statuses)) {
				if (st.jobId) m[path] = st.jobId
			}
		}
		return m
	})

	// Final per-node status (last frame) — used before Play and after the
	// animation completes so node badges reflect the recorded outcome.
	let finalStatuses = $derived.by(() => {
		const frames = recording.timeline
		return frames.length > 0 ? frames[frames.length - 1].statuses : {}
	})

	// Statuses actually shown on the canvas: the animation snapshot while
	// playing, otherwise the recorded final outcome.
	let shownStatuses = $derived(replayState === 'playing' ? currentStatuses : finalStatuses)

	let activeRunnableIds = $derived.by(() => {
		const s = new Set<string>()
		for (const [path, st] of Object.entries(shownStatuses)) {
			if (st.status === 'running') s.add(`script:${path}`)
		}
		return s
	})

	let runStates = $derived.by(() => {
		const m = new Map<string, RunnableRunState>()
		for (const [path, st] of Object.entries(shownStatuses)) {
			if (st.status === 'running') m.set(`script:${path}`, { status: 'running', runs: 0 })
			else if (st.status === 'success') m.set(`script:${path}`, { status: 'success', runs: 1 })
			else if (st.status === 'failure') m.set(`script:${path}`, { status: 'failure', runs: 1 })
		}
		return m
	})

	function clearTimeouts() {
		timeouts.forEach((t) => clearTimeout(t))
		timeouts = []
	}

	/** Offset a recorded job's absolute timestamps to "now" so duration/elapsed
	 * displays compute correctly during replay. */
	function rebaseJobTimestamps(rec: RecordedJob) {
		const anchor = rec.initial_job?.started_at ?? rec.initial_job?.created_at
		if (!anchor) return
		const earliest = new Date(anchor).getTime()
		if (isNaN(earliest)) return
		const offset = Date.now() - earliest
		const offsetDate = (d: string | undefined): string | undefined => {
			if (!d) return d
			const t = new Date(d).getTime()
			return isNaN(t) ? d : new Date(t + offset).toISOString()
		}
		const fix = (j: any) => {
			if (!j) return
			if (j.started_at) j.started_at = offsetDate(j.started_at)
			if (j.created_at) j.created_at = offsetDate(j.created_at)
			if (j.completed_at) j.completed_at = offsetDate(j.completed_at)
		}
		fix(rec.initial_job)
		for (const e of rec.events) fix((e.data as any)?.job)
	}

	// The recorded job id for the selected runnable, if it ran.
	let selectedJobId = $derived.by(() => {
		if (selection?.kind !== 'runnable') return undefined
		const jobId = pathToJobId[selection.path]
		return jobId && recording.jobs[jobId] ? jobId : undefined
	})

	export function startReplay() {
		clearTimeouts()
		currentStatuses = {}
		selection = undefined
		job = undefined
		replayState = 'playing'
		// Put JobLoader into replay mode (no network) even before a node is opened.
		setActiveReplay({ jobs: {} })
		const frames = recording.timeline
		for (const frame of frames) {
			timeouts.push(
				setTimeout(() => {
					currentStatuses = frame.statuses
				}, frame.t)
			)
		}
		const last = frames.length > 0 ? frames[frames.length - 1].t : 0
		timeouts.push(
			setTimeout(() => {
				replayState = 'done'
			}, last + 50)
		)
	}

	export function stop() {
		clearTimeouts()
		setActiveReplay(undefined)
		replayState = 'loaded'
		currentStatuses = {}
		selection = undefined
		job = undefined
	}

	async function openNodeDetail(jobId: string) {
		job = undefined
		const rec = recording.jobs[jobId]
		if (!rec) return
		const cloned = JSON.parse(JSON.stringify(rec)) as RecordedJob
		// Normalize event offsets so the node's stream plays from the moment it
		// is opened (recorded `t` is relative to the whole-pipeline start).
		const base = cloned.events[0]?.t ?? 0
		for (const e of cloned.events) e.t = Math.max(0, e.t - base)
		rebaseJobTimestamps(cloned)
		setActiveReplay({ jobs: { [jobId]: cloned } })
		await tick()
		jobLoader?.watchJob(jobId)
	}

	// When the selected node's recorded job changes, (re)start its replay stream.
	// `openNodeDetail` is imperative (drives the JobLoader ref), so it runs in
	// `untrack` — the effect only re-fires on `selectedJobId` changes.
	$effect(() => {
		const jobId = selectedJobId
		if (jobId) {
			untrack(() => openNodeDetail(jobId))
		} else {
			job = undefined
		}
	})

	let selectedStatus = $derived.by(() => {
		if (selection?.kind !== 'runnable') return undefined
		return (shownStatuses[selection.path] ?? finalStatuses[selection.path])?.status
	})

	onDestroy(() => {
		clearTimeouts()
		setActiveReplay(undefined)
	})
</script>

<div class="flex flex-col gap-4 h-full">
	{#if !hideControls}
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				<h2 class="text-lg font-semibold text-emphasis">
					{replayState === 'playing' ? 'Replaying: ' : ''}{recording.folder || 'Pipeline'}
				</h2>
				<Tooltip placement="bottom">
					<InfoIcon size={16} class="text-tertiary" />
					{#snippet text()}
						<span class="text-2xs">
							Recorded {new Date(recording.recorded_at).toLocaleString()} &mdash;
							{(recording.total_duration_ms / 1000).toFixed(1)}s &mdash;
							{Object.keys(recording.jobs).length} job(s)
						</span>
					{/snippet}
				</Tooltip>
			</div>
			{#if replayState === 'playing'}
				<Button variant="border" size="xs" onclick={stop} startIcon={{ icon: Square }}>Stop</Button>
			{:else if replayState === 'done'}
				<Button variant="border" size="xs" onclick={stop} startIcon={{ icon: LogOut }}>Exit</Button>
			{:else}
				<Button variant="contained" color="blue" onclick={startReplay} startIcon={{ icon: Play }}>
					Play
				</Button>
			{/if}
		</div>
	{/if}

	<!-- Always mounted: a node's recorded detail can be inspected before/after
	     Play, and openNodeDetail drives this loader's `watchJob` in replay mode. -->
	<JobLoader noCode={true} bind:this={jobLoader} bind:job />

	<div class="rounded-md border bg-surface" style="height: 460px;">
		<AssetGraphCanvas
			graph={recording.graph}
			{selection}
			onselect={(s) => (selection = s)}
			{activeRunnableIds}
			{runStates}
			viewportFitKey={recording.folder}
		/>
	</div>

	{#if selection?.kind === 'runnable'}
		<div class="flex flex-col gap-2 rounded-md border bg-surface p-3 min-h-0">
			<div class="flex items-center gap-2">
				{#if selectedStatus === 'running'}
					<Loader2 size={16} class="text-blue-500 animate-spin" />
				{:else if selectedStatus === 'success'}
					<CheckCircle2 size={16} class="text-green-600" />
				{:else if selectedStatus === 'failure'}
					<XCircle size={16} class="text-red-600" />
				{/if}
				<span class="text-xs font-mono text-emphasis break-all">{selection.path}</span>
			</div>

			{#if selectedJobId && recording.jobs[selectedJobId]}
				{#if job?.args && Object.keys(job.args).length > 0}
					<div>
						<h3 class="text-2xs font-semibold text-tertiary mb-1">Arguments</h3>
						<JobArgs args={job.args} />
					</div>
				{/if}
				<div class="grid grid-cols-2 gap-3">
					<div class="flex flex-col min-h-0">
						<h3 class="text-2xs font-semibold text-tertiary mb-1">Logs</h3>
						<div class="h-64 overflow-auto rounded-md border bg-surface-tertiary">
							<LogViewer
								jobId={job?.id}
								duration={job?.['duration_ms']}
								mem={job?.['mem_peak']}
								isLoading={!jobDone}
								content={job?.logs}
								tag={job?.tag}
								download={false}
							/>
						</div>
					</div>
					<div class="flex flex-col min-h-0">
						<h3 class="text-2xs font-semibold text-tertiary mb-1">Result</h3>
						<div class="h-64 overflow-auto rounded-md border bg-surface-tertiary p-3">
							{#if job !== undefined && job.type === 'CompletedJob' && job.result !== undefined}
								<DisplayResult result={job.result} language={job.language} />
							{:else if jobDone}
								<div class="text-secondary text-xs">No output available</div>
							{:else}
								<div class="text-secondary text-xs">Waiting for result…</div>
							{/if}
						</div>
					</div>
				</div>
			{:else}
				<p class="text-xs text-secondary">This node did not run in the recorded session.</p>
			{/if}
		</div>
	{:else}
		<p class="text-2xs text-tertiary">
			{replayState === 'playing'
				? 'Replaying the recorded run — click a node to inspect its logs and result.'
				: 'Click a node to inspect its recorded logs and result.'}
		</p>
	{/if}
</div>
