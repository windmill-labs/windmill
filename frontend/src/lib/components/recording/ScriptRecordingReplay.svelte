<script lang="ts">
	import type { Job, Script } from '$lib/gen'
	import { setActiveReplay } from './replay.svelte'
	import { synthesizeSingleJobReplay } from './replayStream'
	import type { ScriptRecording } from './types'
	import { sendUserToast } from '$lib/toast'
	import { Button, Tab, TabContent } from '$lib/components/common'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { scriptLangToEditorLang } from '$lib/scripts'
	import { Highlight } from 'svelte-highlight'
	import { json as jsonLang } from 'svelte-highlight/languages'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import { ClipboardCopy, InfoIcon, Loader2, LogOut, Play, Square } from 'lucide-svelte'
	import { copyToClipboard } from '$lib/utils'
	import { onDestroy, tick } from 'svelte'
	import JobLoader from '$lib/components/JobLoader.svelte'

	export type ScriptTabValue = 'parameters' | 'code' | 'args' | 'schema' | 'result'

	type ReplayState = 'loaded' | 'playing'

	interface Props {
		recording: ScriptRecording
		selectedTab?: ScriptTabValue
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

	let jobId: string | undefined = $state(undefined)
	let job: Job | undefined = $state(undefined)
	let jobLoader: JobLoader | undefined = $state(undefined)
	let done = $derived((job as any)?.type === 'CompletedJob')

	let schema = $derived(recording.schema)

	let hasInputs = $derived(!!recording.args && Object.keys(schema?.properties ?? {}).length > 0)

	if (selectedTab === undefined) {
		if (hasInputs) selectedTab = 'parameters'
		else if (recording.args && Object.keys(recording.args).length > 0) selectedTab = 'args'
		else selectedTab = 'code'
	}
	if (replayState === undefined) {
		replayState = 'loaded'
	}

	export function stop() {
		setActiveReplay(undefined)
		job = undefined
		replayState = 'loaded'
	}

	function initRecording() {
		const id = recording.job?.id
		if (!id) {
			sendUserToast('Recording has no job data', true)
			return
		}
		jobId = id
		replayState = 'loaded'
	}

	initRecording()

	export async function startReplay() {
		if (!jobId) return
		try {
			// A malformed recording (upload / `?src=` fetch) can throw while the
			// job is walked here — an event-handler throw a Svelte boundary can't
			// catch, so it's guarded rather than left to break the page.
			const stream = synthesizeSingleJobReplay(recording.job)
			setActiveReplay({ jobs: { [jobId]: stream } })
		} catch {
			setActiveReplay(undefined)
			sendUserToast('This recording could not be replayed — it may be malformed', true)
			return
		}
		job = undefined
		replayState = 'playing'
		selectedTab = 'result'
		await tick()
		if (jobLoader && jobId) {
			jobLoader.watchJob(jobId)
		}
	}

	onDestroy(() => {
		setActiveReplay(undefined)
	})
</script>

<HighlightTheme />

{#if !recording?.job?.id}
	<div class="flex flex-col items-center justify-center min-h-[60vh]">
		<div class="border rounded-lg p-8 bg-surface-tertiary max-w-md w-full text-center">
			<p class="text-xs text-secondary">
				This recording does not contain valid job data. It may have been recorded incorrectly.
			</p>
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-4 h-full">
		{#if !hideControls}
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-2">
					<h2 class="text-lg font-semibold text-emphasis">
						{replayState === 'playing' ? 'Replaying: ' : ''}{recording.script_path ||
							'Untitled script'}
					</h2>
					<span class="text-xs text-secondary px-2 py-0.5 bg-surface-secondary rounded">
						{recording.language}
					</span>
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

		{#if replayState === 'playing'}
			<JobLoader noCode={true} bind:this={jobLoader} bind:job />
		{/if}

		<Tabs bind:selected={selectedTab as string} {hideTabs}>
			{#if replayState === 'playing'}
				<Tab value="result" label="Result" />
			{/if}
			{#if hasInputs}
				<Tab value="parameters" label="Inputs" />
			{/if}
			{#if !hasInputs && recording.args && Object.keys(recording.args).length > 0}
				<Tab value="args" label="Args" />
			{/if}
			<Tab value="code" label="Code" />
			{#if schema}
				<Tab value="schema" label="Schema" />
			{/if}
			{#snippet content()}
				<TabContent value="result" class="flex-1 min-h-0">
					{#if replayState === 'playing' && jobId}
						<div class="grid grid-cols-2 gap-4 w-full h-full">
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
							<div class="flex flex-col min-h-0">
								<h3 class="shrink-0 text-xs font-semibold text-emphasis mb-1">Result</h3>
								<div class="flex-1 min-h-0 overflow-auto rounded-md border bg-surface-tertiary p-4">
									{#if job !== undefined && job.type === 'CompletedJob' && job.result !== undefined}
										<DisplayResult result={job.result} language={job.language} />
									{:else if done}
										<div
											class="w-full h-full flex items-center justify-center text-secondary text-sm"
										>
											No output available
										</div>
									{:else}
										<div
											class="w-full h-full flex items-center justify-center text-secondary text-sm"
										>
											Waiting for result...
										</div>
									{/if}
								</div>
							</div>
						</div>
					{/if}
				</TabContent>
				<TabContent value="parameters" class="flex-1 min-h-0">
					{#if schema && recording.args}
						<div class="p-2 h-full overflow-auto">
							<SchemaForm {schema} args={recording.args} disabled={true} noVariablePicker={true} />
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
				<TabContent value="code" class="flex-1 min-h-0">
					<div class="h-full min-h-[300px] overflow-hidden rounded-md border">
						{#await import('$lib/components/SimpleEditor.svelte')}
							<Loader2 size={16} class="animate-spin m-4 text-secondary" />
						{:then { default: SimpleEditor }}
							<SimpleEditor
								class="h-full"
								lang={scriptLangToEditorLang(recording.language as Script['language'])}
								code={recording.code}
								readOnly
								disableSuggestions
								disableLinting
							/>
						{/await}
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
{/if}
