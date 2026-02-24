<script lang="ts">
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import FlowExecutionStatus from '$lib/components/runs/FlowExecutionStatus.svelte'
	import { setActiveReplay } from './flowRecording.svelte'
	import type { FlowRecording } from './types'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { InfoIcon, LogOut, Play, Square } from 'lucide-svelte'
	import { onDestroy } from 'svelte'

	interface Props {
		recording: FlowRecording
	}

	let { recording }: Props = $props()

	type ReplayState = 'loaded' | 'playing'

	let replayState: ReplayState = $state('loaded')
	let rootJobId: string | undefined = $state(undefined)
	let rootInitialJob: Job | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)
	let done = $derived((job as any)?.type === 'CompletedJob')

	function stop() {
		setActiveReplay(undefined)
		job = undefined
		initRecording()
	}

	function findRootJobId(data: FlowRecording): string | undefined {
		for (const [id, recorded] of Object.entries(data.jobs)) {
			const j = recorded.initial_job
			if (
				(j.job_kind === 'flow' || j.job_kind === 'flowpreview') &&
				!j.parent_job
			) {
				return id
			}
		}
		for (const [id, recorded] of Object.entries(data.jobs)) {
			if (!recorded.initial_job.parent_job) return id
		}
		return Object.keys(data.jobs)[0]
	}

	/**
	 * Offset all absolute timestamps in the recording so they are relative to "now".
	 * TimelineCompute uses Date.now() for in-progress bars, so we need recorded
	 * started_at/created_at values to be near the current time.
	 */
	function rebaseTimestamps(data: FlowRecording, rootId: string): FlowRecording {
		const rootJob = data.jobs[rootId]?.initial_job
		const anchor = rootJob?.started_at ?? rootJob?.created_at
		if (!anchor) return data
		const earliest = new Date(anchor).getTime()
		if (isNaN(earliest)) return data

		const offset = Date.now() - earliest

		function offsetDate(d: string | number | undefined): string | undefined {
			if (!d) return d as undefined
			const t = new Date(d).getTime()
			if (isNaN(t)) return d as string
			return new Date(t + offset).toISOString()
		}

		function offsetJobTimestamps(j: any) {
			if (j.started_at) j.started_at = offsetDate(j.started_at)
			if (j.created_at) j.created_at = offsetDate(j.created_at)
			if (j.completed_at) j.completed_at = offsetDate(j.completed_at)
		}

		function offsetFlowStatus(fs: any) {
			if (!fs?.modules) return
			for (const mod of fs.modules) {
				const durations = mod.flow_jobs_duration
				if (durations?.started_at) {
					durations.started_at = durations.started_at.map(
						(d: string) => offsetDate(d) ?? d
					)
				}
			}
		}

		for (const recorded of Object.values(data.jobs)) {
			offsetJobTimestamps(recorded.initial_job)
			for (const event of recorded.events) {
				if (event.data?.job) offsetJobTimestamps(event.data.job)
				if (event.data?.flow_status) offsetFlowStatus(event.data.flow_status)
			}
		}
		return data
	}

	function buildInitialJob(data: FlowRecording, jobId: string): Job {
		const initialJob = JSON.parse(JSON.stringify(data.jobs[jobId].initial_job))
		if (data.flow?.value) {
			initialJob.raw_flow = JSON.parse(JSON.stringify(data.flow.value))
		}
		return initialJob
	}

	function initRecording() {
		const flowJobId = findRootJobId(recording)
		if (!flowJobId) {
			sendUserToast('Recording has no jobs', true)
			return
		}
		rootJobId = flowJobId
		rootInitialJob = buildInitialJob(recording, flowJobId)
		replayState = 'loaded'
	}

	// Initialize on mount
	initRecording()

	/**
	 * Ensure the root flow's completed event fires after all sub-job events.
	 * During recording, addCompletedJob runs after the flow completes, so sub-job
	 * completed events can have a later `t` than the root's completed event.
	 */
	function fixEventOrdering(data: FlowRecording, rootId: string) {
		const rootEvents = data.jobs[rootId]?.events
		if (!rootEvents?.length) return

		// Find the latest event.t across all sub-jobs
		let maxSubJobT = 0
		for (const [id, recorded] of Object.entries(data.jobs)) {
			if (id === rootId) continue
			for (const event of recorded.events) {
				if (event.t > maxSubJobT) maxSubJobT = event.t
			}
		}

		// Push the root's completed event to fire after all sub-job events
		let completedIdx = -1
		for (let i = rootEvents.length - 1; i >= 0; i--) {
			if (rootEvents[i].data.completed) { completedIdx = i; break }
		}
		if (completedIdx >= 0 && rootEvents[completedIdx].t < maxSubJobT) {
			rootEvents[completedIdx].t = maxSubJobT + 50
		}
	}

	function startReplay() {
		// JSON round-trip to unwrap reactive proxies and strip non-cloneable properties
		const snapshot = JSON.parse(JSON.stringify(recording)) as FlowRecording
		fixEventOrdering(snapshot, rootJobId!)
		rebaseTimestamps(snapshot, rootJobId!)
		setActiveReplay(snapshot)
		rootInitialJob = buildInitialJob(snapshot, rootJobId!)
		job = undefined
		replayState = 'playing'
	}

	onDestroy(() => {
		setActiveReplay(undefined)
	})
</script>

{#if !recording?.flow}
	<div class="flex flex-col items-center justify-center min-h-[60vh]">
		<div class="border rounded-lg p-8 bg-surface-tertiary max-w-md w-full text-center">
			<p class="text-xs text-secondary">
				This recording does not include a flow definition. It was likely recorded with an older
				version. Re-record the flow to include the flow definition.
			</p>
		</div>
	</div>
{:else if replayState === 'loaded'}
	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				<h2 class="text-lg font-semibold text-emphasis">{recording.flow_path}</h2>
				<Tooltip placement="bottom">
					<InfoIcon size={16} class="text-tertiary" />
					<span class="text-2xs" slot="text">
						Recorded {new Date(recording.recorded_at).toLocaleString()} &mdash;
						{(recording.total_duration_ms / 1000).toFixed(1)}s
					</span>
				</Tooltip>
			</div>
			<Button variant="contained" color="blue" on:click={startReplay} startIcon={{ icon: Play }}>
				Play
			</Button>
		</div>
		<FlowViewer flow={recording.flow} noSummary />
	</div>
{:else if replayState === 'playing' && rootJobId}
	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-emphasis">Replaying: {recording.flow_path}</h2>
			<Button variant="border" size="xs" on:click={stop} startIcon={{ icon: done ? LogOut : Square }}>
				{done ? 'Exit' : 'Stop'}
			</Button>
		</div>
		<FlowProgressBar {job} slim textPosition="bottom" showStepId />
		{#if job}
			<FlowExecutionStatus
				{job}
				workspaceId={$workspaceStore}
				isOwner={false}
				innerModules={job?.flow_status?.modules}
				suspendStatus={{ val: {} }}
			/>
		{/if}
		<FlowStatusViewer
			jobId={rootJobId}
			initialJob={rootInitialJob}
			bind:job
			workspaceId={$workspaceStore}
			wideResults
			showLogsWithResult
		/>
	</div>
{/if}
