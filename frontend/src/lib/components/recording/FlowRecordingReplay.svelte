<script lang="ts">
	import type { Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import FlowViewer, { type TabValue } from '$lib/components/FlowViewer.svelte'
	import FlowGraphViewer from '$lib/components/FlowGraphViewer.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import FlowExecutionStatus from '$lib/components/runs/FlowExecutionStatus.svelte'
	import { setActiveReplay } from './replay.svelte'
	import { synthesizeFlowReplay } from './replayStream'
	import type { FlowRecording } from './types'
	import { sendUserToast } from '$lib/toast'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { InfoIcon, LogOut, Play, Square } from 'lucide-svelte'
	import { onDestroy } from 'svelte'

	type ReplayState = 'loaded' | 'playing'

	interface Props {
		recording: FlowRecording
		selectedTab?: TabValue
		replayState?: ReplayState
		hideControls?: boolean
		hideTabs?: boolean
	}

	let {
		recording,
		selectedTab = $bindable(),
		replayState = $bindable(),
		hideControls = false,
		hideTabs = false
	}: Props = $props()

	if (selectedTab === undefined) {
		selectedTab = 'ui'
	}
	if (replayState === undefined) {
		replayState = 'loaded'
	}

	let rootJobId: string | undefined = $state(undefined)
	let rootInitialJob: Job | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)
	let done = $derived((job as any)?.type === 'CompletedJob')

	export function stop() {
		setActiveReplay(undefined)
		job = undefined
		replayState = 'loaded'
	}

	function initRecording() {
		const flowJobId = recording.root_job_id
		if (!flowJobId || !recording.jobs?.[flowJobId]) {
			sendUserToast('Recording has no jobs', true)
			return
		}
		rootJobId = flowJobId
		replayState = 'loaded'
	}

	// Initialize on mount
	initRecording()

	export function startReplay() {
		if (!rootJobId) return
		try {
			// A malformed recording (upload / `?src=` fetch) can throw while its
			// jobs' flow_status is walked here — an event-handler throw a Svelte
			// boundary can't catch, so it's guarded rather than left to break the page.
			const replay = synthesizeFlowReplay(recording.jobs, rootJobId)
			setActiveReplay(replay)
			const initialJob = JSON.parse(JSON.stringify(replay.jobs[rootJobId].initial_job))
			if (recording.flow?.value) {
				initialJob.raw_flow = JSON.parse(JSON.stringify(recording.flow.value))
			}
			rootInitialJob = initialJob
			job = undefined
			replayState = 'playing'
			selectedTab = 'ui'
		} catch {
			setActiveReplay(undefined)
			replayState = 'loaded'
			sendUserToast('This recording could not be replayed — it may be malformed', true)
		}
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
{:else}
	<div class="flex flex-col gap-4">
		{#if !hideControls}
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<h2 class="text-lg font-semibold text-emphasis">
						{replayState === 'playing' ? 'Replaying: ' : ''}{recording.flow_path}
					</h2>
					<Tooltip placement="bottom">
						<InfoIcon size={16} class="text-tertiary" />
						{#snippet text()}
							<span class="text-2xs">
								Recorded {new Date(recording.recorded_at).toLocaleString()} &mdash;
								{(recording.total_duration_ms / 1000).toFixed(1)}s
							</span>
						{/snippet}
					</Tooltip>
				</div>
				{#if replayState === 'loaded'}
					<Button variant="contained" color="blue" onclick={startReplay} startIcon={{ icon: Play }}>
						Play
					</Button>
				{:else}
					<Button
						variant="border"
						size="xs"
						onclick={stop}
						startIcon={{ icon: done ? LogOut : Square }}
					>
						{done ? 'Exit' : 'Stop'}
					</Button>
				{/if}
			</div>
		{/if}

		<FlowViewer
			flow={recording?.flow}
			noSummary
			noInput
			hideDefaultInputs
			showStepHint={replayState === 'loaded'}
			bind:selectedTab
			{hideTabs}
			initTab="ui"
		>
			{#snippet graphContent()}
				{#if replayState === 'playing' && rootJobId}
					<div class="flex flex-col gap-4">
						<FlowProgressBar {job} slim textPosition="bottom" showStepId />
						{#if job}
							<FlowExecutionStatus
								{job}
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
							hideFlowResult={!done}
						/>
					</div>
				{:else if recording?.flow}
					<div class="flow-root w-full pb-4">
						<p class="text-2xs text-tertiary py-1">Click on a step to see its details</p>
						<FlowGraphViewer hideDefaultInputs flow={recording?.flow} overflowAuto />
					</div>
				{/if}
			{/snippet}
		</FlowViewer>
	</div>
{/if}
