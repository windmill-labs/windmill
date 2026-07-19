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
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import {
		CheckCircle2,
		Database,
		InfoIcon,
		Loader2,
		LogOut,
		Play,
		RotateCcw,
		Square,
		XCircle
	} from 'lucide-svelte'
	import { onDestroy, tick, untrack } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

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

	// Graph/detail split, mirroring PipelineGraphEditor: the graph keeps the full
	// height and the detail pane only mounts on selection. Below `STACK_BELOW` the
	// split flips to vertical, where the detail pane needs the larger share.
	const STACK_BELOW = 680
	let containerWidth = $state(0)
	let stacked = $derived(containerWidth > 0 && containerWidth < STACK_BELOW)
	let leftPaneSize = $state(100)
	let rightPaneSize = $state(0)
	// 0 = no user-chosen size yet, so the orientation-aware default applies.
	let storedRightPaneSize = $state(0)
	// Depend on the open/closed transition, not on `selection` itself — selecting
	// another node must not throw away a size the user dragged.
	let detailsOpen = $derived(selection != undefined)

	// untrack the writes (and the rightPaneSize read) so the effect tracks only
	// `detailsOpen`/`stacked` — a tracked `bind:size` read feeds back through Pane
	// and pegs the main thread.
	$effect(() => {
		const fallback = stacked ? 55 : 40
		if (detailsOpen) {
			untrack(() => {
				const size = storedRightPaneSize > 0 ? storedRightPaneSize : fallback
				rightPaneSize = size
				leftPaneSize = 100 - size
			})
		} else {
			untrack(() => {
				if (rightPaneSize > 0) storedRightPaneSize = rightPaneSize
				rightPaneSize = 0
				leftPaneSize = 100
			})
		}
	})

	let jobDone = $derived((job as any)?.type === 'CompletedJob')

	// Runnable path -> the asset node ids (`asset:${kind}:${path}`) it writes,
	// from the graph's write/rw edges. Lets a producer's success flash the green
	// "recomputed" wash on exactly the tables it materialized.
	let producerToAssets = $derived.by(() => {
		const m = new Map<string, string[]>()
		for (const e of recording.graph.edges ?? []) {
			const access = e.access_type ?? 'r'
			if (access !== 'w' && access !== 'rw') continue
			const assetId = `asset:${e.asset_kind}:${e.asset_path}`
			const list = m.get(e.runnable_path) ?? []
			if (!list.includes(assetId)) list.push(assetId)
			m.set(e.runnable_path, list)
		}
		return m
	})

	// asset node id -> a monotonic nonce, bumped each time the replay reaches the
	// frame where the asset's producer turned successful. The canvas hands each
	// asset node its nonce; a change replays the one-shot green fade.
	let recomputePulses = $state<Record<string, number>>({})
	let recomputedAssetIds = $derived(new Map(Object.entries(recomputePulses)))

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
		recomputePulses = {}
		selection = undefined
		job = undefined
		replayState = 'playing'
		// Put JobLoader into replay mode (no network) even before a node is opened.
		setActiveReplay({ jobs: {} })
		const frames = recording.timeline
		let pulseCounter = 0
		for (let i = 0; i < frames.length; i++) {
			const frame = frames[i]
			const prev = i > 0 ? frames[i - 1].statuses : {}
			// Assets whose producer transitions into `success` at this frame — they
			// were just (re)materialized, so pulse them green as the frame lands.
			const pulsedAssets: string[] = []
			for (const [path, st] of Object.entries(frame.statuses)) {
				if (st.status === 'success' && prev[path]?.status !== 'success') {
					for (const a of producerToAssets.get(path) ?? []) pulsedAssets.push(a)
				}
			}
			timeouts.push(
				setTimeout(() => {
					currentStatuses = frame.statuses
					for (const a of pulsedAssets) recomputePulses[a] = ++pulseCounter
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
		recomputePulses = {}
		selection = undefined
		job = undefined
	}

	async function openNodeDetail(jobId: string) {
		job = undefined
		const rec = recording.jobs[jobId]
		if (!rec) return
		const cloned = JSON.parse(JSON.stringify(rec)) as RecordedJob
		// Only a node that is still running in the animation replays its stream
		// live; a node the graph already shows as finished reveals its logs and
		// result at once (collapse every event to t=0) rather than re-playing the
		// whole execution as if it were computing now. Recorded `t` is relative to
		// the whole-pipeline start, so rebase to the moment of open for the live case.
		const streamLive = selectedStatus === 'running'
		const base = cloned.events[0]?.t ?? 0
		for (const e of cloned.events) e.t = streamLive ? Math.max(0, e.t - base) : 0
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

	// Output (args/logs/result) vs the step's source code, for a runnable node.
	let runnableTab = $state<'output' | 'code'>('output')
	let selectedCode = $derived.by(() => {
		if (selection?.kind !== 'runnable') return undefined
		return recording.codes?.[selection.path]
	})

	// Recorded data-sample for a selected asset node (ducklake/datatable).
	let selectedAssetSample = $derived.by(() => {
		if (selection?.kind !== 'asset') return undefined
		return recording.assetSamples?.[`${selection.asset_kind}:${selection.path}`]
	})

	// Render a cell value: primitives as-is, objects/arrays as compact JSON.
	function fmtCell(v: unknown): string {
		if (v === null || v === undefined) return ''
		if (typeof v === 'object') return JSON.stringify(v)
		return String(v)
	}

	onDestroy(() => {
		clearTimeouts()
		setActiveReplay(undefined)
	})
</script>

<div class="flex flex-col gap-2 h-full min-h-0">
	{#if !hideControls}
		<div class="flex items-center justify-between shrink-0">
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
				<div class="flex items-center gap-2">
					<Button variant="border" size="xs" onclick={stop} startIcon={{ icon: LogOut }}
						>Exit</Button
					>
					<Button
						variant="contained"
						color="blue"
						onclick={startReplay}
						startIcon={{ icon: RotateCcw }}
					>
						Replay
					</Button>
				</div>
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

	<div
		class="flex-1 min-h-0 rounded-md border bg-surface overflow-hidden"
		bind:clientWidth={containerWidth}
	>
		<Splitpanes class="!h-full" horizontal={stacked}>
			<Pane bind:size={leftPaneSize}>
				<div class="relative h-full">
					<AssetGraphCanvas
						graph={recording.graph}
						{selection}
						onselect={(s) => (selection = s)}
						{activeRunnableIds}
						{runStates}
						highlightActiveRun={replayState === 'playing'}
						{recomputedAssetIds}
						showMinimap={!stacked}
						viewportFitKey={recording.folder}
					/>
					{#if !selection}
						<p
							class="absolute bottom-2 left-1/2 -translate-x-1/2 z-20 px-2 py-1 rounded-md bg-surface/95 backdrop-blur-sm border text-2xs text-tertiary shadow-sm"
						>
							{replayState === 'playing'
								? 'Replaying the recorded run — click a node to inspect its logs, result or data sample.'
								: 'Click a script node for its logs/result, or an asset node for its recorded data sample.'}
						</p>
					{/if}
				</div>
			</Pane>
			{#if selection}
				<Pane bind:size={rightPaneSize} minSize={25}>
					<div class="h-full min-h-0">
						{#if selection.kind === 'runnable'}
							<div class="flex flex-col gap-2 p-3 h-full min-h-0">
								<div class="flex items-center gap-2 shrink-0">
									{#if selectedStatus === 'running'}
										<Loader2 size={16} class="text-blue-500 animate-spin" />
									{:else if selectedStatus === 'success'}
										<CheckCircle2 size={16} class="text-green-600" />
									{:else if selectedStatus === 'failure'}
										<XCircle size={16} class="text-red-600" />
									{/if}
									<span class="text-xs font-mono text-emphasis break-all">{selection.path}</span>
									<div class="ml-auto shrink-0">
										<ToggleButtonGroup
											selected={runnableTab}
											on:selected={(e) => (runnableTab = e.detail)}
										>
											{#snippet children({ item })}
												<ToggleButton size="xs" value="output" label="Output" {item} />
												<ToggleButton size="xs" value="code" label="Code" {item} />
											{/snippet}
										</ToggleButtonGroup>
									</div>
								</div>

								{#if runnableTab === 'code'}
									<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary">
										{#if selectedCode}
											<HighlightCode
												code={selectedCode.content}
												language={selectedCode.language as any}
											/>
										{:else}
											<p class="text-xs text-secondary p-3">
												No code was captured for this step in the recording.
											</p>
										{/if}
									</div>
								{:else if selectedJobId && recording.jobs[selectedJobId]}
									{#if job?.args && Object.keys(job.args).length > 0}
										<div class="shrink-0">
											<h3 class="text-2xs font-semibold text-tertiary mb-1">Arguments</h3>
											<JobArgs args={job.args} />
										</div>
									{/if}
									<!-- Logs and Result share the leftover height; Result gets the
									     larger share (2:1) since a pipeline step's output is what you
									     usually inspect, while both stay scrollable when squeezed. -->
									<div class="flex flex-col flex-1 min-h-0">
										<h3 class="text-2xs font-semibold text-tertiary mb-1">Logs</h3>
										<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary">
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
									<div class="flex flex-col flex-[2_1_0%] min-h-0">
										<h3 class="text-2xs font-semibold text-tertiary mb-1">Result</h3>
										<div
											class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary p-3"
										>
											{#if job !== undefined && job.type === 'CompletedJob' && job.result !== undefined}
												<DisplayResult result={job.result} language={job.language} />
											{:else if jobDone}
												<div class="text-secondary text-xs">No output available</div>
											{:else}
												<div class="text-secondary text-xs">Waiting for result…</div>
											{/if}
										</div>
									</div>
								{:else}
									<p class="text-xs text-secondary"
										>This node did not run in the recorded session.</p
									>
								{/if}
							</div>
						{:else if selection.kind === 'asset'}
							<div class="flex flex-col gap-2 p-3 h-full min-h-0">
								<div class="flex items-center gap-2">
									<Database size={16} class="text-tertiary" />
									<span class="text-xs font-mono text-emphasis break-all"
										>{selectedAssetSample?.uri ??
											`${selection.asset_kind}://${selection.path}`}</span
									>
									{#if selectedAssetSample && !selectedAssetSample.error}
										<span class="text-2xs text-tertiary">
											{selectedAssetSample.rowCount != undefined
												? `${selectedAssetSample.rowCount} row${selectedAssetSample.rowCount === 1 ? '' : 's'}`
												: `${selectedAssetSample.rows.length} sampled`}
											· {selectedAssetSample.columns.length} column{selectedAssetSample.columns
												.length === 1
												? ''
												: 's'}
										</span>
									{/if}
								</div>

								{#if !selectedAssetSample}
									<p class="text-xs text-secondary">
										No data sample was captured for this asset in the recorded session.
									</p>
								{:else if selectedAssetSample.error}
									<p class="text-xs text-secondary">
										Could not capture a sample of this table: {selectedAssetSample.error}
									</p>
								{:else if selectedAssetSample.rows.length === 0}
									<p class="text-xs text-secondary"
										>This table was empty when the recording was taken.</p
									>
								{:else}
									<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary">
										<table class="text-2xs w-full border-collapse">
											<thead class="sticky top-0 bg-surface-secondary">
												<tr>
													{#each selectedAssetSample.columns as c}
														<th
															class="text-left px-2 py-1 font-semibold border-b whitespace-nowrap"
														>
															{c.field}
															{#if c.datatype}<span class="text-tertiary font-normal"
																	>· {c.datatype}</span
																>{/if}
														</th>
													{/each}
												</tr>
											</thead>
											<tbody>
												{#each selectedAssetSample.rows as row}
													<tr class="border-b last:border-b-0">
														{#each selectedAssetSample.columns as c}
															<td class="px-2 py-1 font-mono whitespace-nowrap max-w-xs truncate">
																{fmtCell((row as any)?.[c.field])}
															</td>
														{/each}
													</tr>
												{/each}
											</tbody>
										</table>
									</div>
								{/if}
							</div>
						{/if}
					</div>
				</Pane>
			{/if}
		</Splitpanes>
	</div>
</div>
