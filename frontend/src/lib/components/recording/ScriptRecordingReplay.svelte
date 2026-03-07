<script lang="ts">
	import type { Job, Script } from '$lib/gen'
	import { setActiveReplay } from './flowRecording.svelte'
	import { createScriptRecording } from './scriptRecording.svelte'
	import type { ScriptRecording } from './types'
	import { sendUserToast } from '$lib/toast'
	import { Button, Tab, TabContent } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Highlight } from 'svelte-highlight'
	import { json as jsonLang } from 'svelte-highlight/languages'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { ClipboardCopy, InfoIcon, LogOut, Play, Square } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import { onDestroy, tick } from 'svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'

	interface Props {
		recording: ScriptRecording
	}

	let { recording }: Props = $props()

	type ReplayState = 'loaded' | 'playing'

	let replayState: ReplayState = $state('loaded')
	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)
	let jobLoader: JobLoader | undefined = $state(undefined)
	let done = $derived((job as any)?.type === 'CompletedJob')

	let scriptRecordingStore = createScriptRecording()

	function stop() {
		setActiveReplay(undefined)
		job = undefined
		replayState = 'loaded'
	}

	/**
	 * Rebase absolute timestamps so they are relative to "now".
	 * JobLoader replay uses Date.now() for delay computation.
	 */
	function rebaseTimestamps(data: ScriptRecording): ScriptRecording {
		const anchor = data.job?.initial_job?.started_at ?? data.job?.initial_job?.created_at
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

		offsetJobTimestamps(data.job.initial_job)
		for (const event of data.job.events) {
			if (event.data?.job) offsetJobTimestamps(event.data.job)
		}
		return data
	}

	function initRecording() {
		const id = recording.job?.initial_job?.id
		if (!id) {
			sendUserToast('Recording has no job data', true)
			return
		}
		jobId = id
		replayState = 'loaded'
	}

	initRecording()

	async function startReplay() {
		const snapshot = JSON.parse(JSON.stringify(recording)) as ScriptRecording
		rebaseTimestamps(snapshot)
		const replayData = scriptRecordingStore.toReplayData(snapshot)
		setActiveReplay(replayData)
		job = undefined
		replayState = 'playing'
		await tick()
		if (jobLoader && jobId) {
			jobLoader.watchJob(jobId)
		}
	}

	onDestroy(() => {
		setActiveReplay(undefined)
	})

	let schema = $derived(recording.schema)
</script>

<HighlightTheme />

{#if !recording?.job?.initial_job?.id}
	<div class="flex flex-col items-center justify-center min-h-[60vh]">
		<div class="border rounded-lg p-8 bg-surface-tertiary max-w-md w-full text-center">
			<p class="text-xs text-secondary">
				This recording does not contain valid job data. It may have been recorded incorrectly.
			</p>
		</div>
	</div>
{:else if replayState === 'loaded'}
	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				<h2 class="text-lg font-semibold text-emphasis"
					>{recording.script_path || 'Untitled script'}</h2
				>
				<span class="text-xs text-secondary px-2 py-0.5 bg-surface-secondary rounded"
					>{recording.language}</span
				>
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
			<Button variant="contained" color="blue" on:click={startReplay} startIcon={{ icon: Play }}>
				Play
			</Button>
		</div>

		<Tabs selected={schema && recording.args ? 'parameters' : recording.args && Object.keys(recording.args).length > 0 ? 'args' : 'code'}>
			{#if schema && recording.args}
				<Tab value="parameters" label="Code" />
			{/if}
			{#if recording.args && Object.keys(recording.args).length > 0}
				<Tab value="args" label="Args" />
			{/if}
			{#if !schema || !recording.args}
				<Tab value="code" label="Code" />
			{/if}
			{#if schema}
				<Tab value="schema" label="Schema" />
			{/if}
			{#snippet content()}
				<TabContent value="parameters">
					{#if schema && recording.args}
						<div class="flex gap-4 p-2" style="height: calc(100vh - 250px);">
							<div class="w-1/2 overflow-auto text-2xs">
								<HighlightCode
									language={recording.language as Script['language']}
									code={recording.code}
									lines
									className="text-2xs"
								/>
							</div>
							<div class="w-1/2 overflow-auto">
								<SchemaForm
									schema={schema}
									args={recording.args}
									disabled={true}
									noVariablePicker={true}
								/>
							</div>
						</div>
					{/if}
				</TabContent>
				<TabContent value="args">
					{#if recording.args && Object.keys(recording.args).length > 0}
						<div class="p-2">
							<JobArgs args={recording.args} />
						</div>
					{/if}
				</TabContent>
				<TabContent value="code">
					<div class="p-2 w-full overflow-auto text-2xs">
						<HighlightCode
							language={recording.language as Script['language']}
							code={recording.code}
							lines
							className="text-2xs"
						/>
					</div>
				</TabContent>
				<TabContent value="schema">
					<div class="p-1 relative overflow-auto text-xs">
						{#if schema}
							<button
								onclick={() => copyToClipboard(JSON.stringify(schema, null, 4))}
								class="absolute top-2 right-2"
							>
								<ClipboardCopy size={14} />
							</button>
							<Highlight language={jsonLang} code={JSON.stringify(schema, null, 4)} />
						{:else}
							<p class="bg-surface-secondary text-sm p-2">
								No schema available in this recording
							</p>
						{/if}
					</div>
				</TabContent>
			{/snippet}
		</Tabs>
	</div>
{:else if replayState === 'playing' && jobId}
	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-emphasis"
				>Replaying: {recording.script_path || 'Untitled script'}</h2
			>
			<Button
				variant="border"
				size="xs"
				on:click={stop}
				startIcon={{ icon: done ? LogOut : Square }}
			>
				{done ? 'Exit' : 'Stop'}
			</Button>
		</div>
		<JobLoader noCode={true} bind:this={jobLoader} bind:job />

		<div class="grid grid-cols-2 gap-4 w-full" style="height: calc(100vh - 200px);">
			<div class="flex flex-col min-h-0">
				<h3 class="shrink-0 text-xs font-semibold text-emphasis mb-1">Result</h3>
				<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary p-4">
					{#if job !== undefined && job.type === 'CompletedJob' && job.result !== undefined}
						<DisplayResult result={job.result} language={job.language} />
					{:else if done}
						<div class="w-full h-full flex items-center justify-center text-secondary text-sm">
							No output available
						</div>
					{:else}
						<div class="w-full h-full flex items-center justify-center text-secondary text-sm">
							Waiting for result...
						</div>
					{/if}
				</div>
			</div>
			<div class="flex flex-col min-h-0">
				<h3 class="shrink-0 text-xs font-semibold text-emphasis mb-1">Logs</h3>
				<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary">
					<LogViewer
						jobId={job?.id}
						duration={job?.['duration_ms']}
						mem={job?.['mem_peak']}
						isLoading={!done}
						content={job?.logs}
						tag={job?.tag}
						download={false}
					/>
				</div>
			</div>
		</div>
	</div>
{/if}
