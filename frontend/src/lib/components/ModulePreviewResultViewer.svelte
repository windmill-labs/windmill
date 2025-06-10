<script lang="ts">
	import LogViewer from './LogViewer.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import ScriptFix from './copilot/ScriptFix.svelte'
	import type DiffEditor from './DiffEditor.svelte'
	import type Editor from './Editor.svelte'
	import type { Script, Job, FlowModule } from '$lib/gen'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type { FlowEditorContext } from './flows/types'
	import { getContext } from 'svelte'
	import { getStringError } from './copilot/chat/utils'

	interface Props {
		lang: Script['language']
		editor: Editor | undefined
		diffEditor: DiffEditor | undefined
		loopStatus?: { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' } | undefined
		lastJob?: Job | undefined
		scriptProgress?: number | undefined
		testJob?: Job | undefined
		mod: FlowModule
		testIsLoading?: boolean
		disableMock?: boolean
		disableHistory?: boolean
	}

	let {
		lang,
		editor,
		diffEditor,
		loopStatus = undefined,
		lastJob = undefined,
		scriptProgress = $bindable(undefined),
		testJob = undefined,
		mod,
		testIsLoading = false,
		disableMock = false,
		disableHistory = false
	}: Props = $props()

	const { testStepStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let selectedJob: Job | undefined = $state(undefined)
	let fetchingLastJob = false
	let preview: 'mock' | 'job' | undefined = $state(undefined)
	let jobProgressReset: () => void = $state(() => {})

	let nlastJob = $derived.by(() => {
		if (lastJob) {
			return { ...lastJob, preview: false }
		}
		if (testJob) {
			return { ...testJob, preview: true }
		}
		return undefined
	})

	let forceJson = $state(false)
</script>

<Splitpanes horizontal>
	<Pane size={50} minSize={10} class="text-sm text-tertiary">
		{#if scriptProgress}
			<JobProgressBar
				job={testJob}
				bind:scriptProgress
				bind:reset={jobProgressReset}
				compact={true}
			/>
		{/if}

		<OutputPickerInner
			lastJob={nlastJob}
			fullResult
			moduleId={mod.id}
			closeOnOutsideClick={true}
			getLogs
			on:updateMock
			mock={mod.mock}
			bind:forceJson
			bind:selectedJob
			isLoading={(testIsLoading && !scriptProgress) || fetchingLastJob}
			bind:preview
			path={`path` in mod.value ? mod.value.path : ''}
			{loopStatus}
			{disableMock}
			{disableHistory}
		>
			{#snippet copilot_fix()}
				{#if lang && editor && diffEditor && $testStepStore[mod.id] && selectedJob?.type === 'CompletedJob' && !selectedJob.success && getStringError(selectedJob.result)}
					<ScriptFix {lang} />
				{/if}
			{/snippet}
		</OutputPickerInner>
	</Pane>
	<Pane size={50} minSize={10}>
		{#if (mod.mock?.enabled && preview != 'job') || preview == 'mock'}
			<LogViewer
				small
				content={undefined}
				isLoading={false}
				tag={undefined}
				customEmptyMessage="Using pinned data"
			/>
		{:else}
			<LogViewer
				small
				jobId={selectedJob?.id}
				duration={selectedJob?.['duration_ms']}
				mem={selectedJob?.['mem_peak']}
				content={selectedJob?.logs}
				isLoading={(testIsLoading && selectedJob?.['running'] == false) || fetchingLastJob}
				tag={selectedJob?.tag}
			/>
		{/if}
	</Pane>
</Splitpanes>
