<script lang="ts">
	import LogViewer from './LogViewer.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import ScriptFix from './copilot/ScriptFix.svelte'
	import type DiffEditor from './DiffEditor.svelte'
	import type Editor from './Editor.svelte'
	import { type Script, type Job, type FlowModule } from '$lib/gen'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type { FlowEditorContext, OutputViewerJob } from './flows/types'
	import { getContext } from 'svelte'
	import { getStringError } from './copilot/chat/utils'
	import AiAgentLogViewer from './AIAgentLogViewer.svelte'

	interface Props {
		lang: Script['language']
		editor: Editor | undefined
		diffEditor: DiffEditor | undefined
		loopStatus?: { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' } | undefined
		testJob?: Job & { result_stream?: string }
		scriptProgress?: number | undefined
		mod: FlowModule
		testIsLoading?: boolean
		disableMock?: boolean
		disableHistory?: boolean
		onUpdateMock?: (mock: { enabled: boolean; return_value?: unknown }) => void
		loadingJob?: boolean
		tagLabel?: string
	}

	let {
		lang,
		editor,
		diffEditor,
		loopStatus = undefined,
		scriptProgress = $bindable(undefined),
		testJob = undefined,
		mod,
		testIsLoading = false,
		disableMock = false,
		disableHistory = false,
		onUpdateMock,
		loadingJob = false,
		tagLabel = undefined
	}: Props = $props()

	const { stepsInputArgs } = getContext<FlowEditorContext>('FlowEditorContext')

	let outputPickerInner: OutputPickerInner | undefined = $state(undefined)
	export function getOutputPickerInner() {
		return outputPickerInner
	}

	const selectedJob: OutputViewerJob = $derived.by(
		() => outputPickerInner?.getSelectedJob?.() ?? undefined
	)
	const logJob = $derived(testJob ?? selectedJob)
	const preview = $derived.by(() => outputPickerInner?.getPreview?.())
</script>

<Splitpanes horizontal>
	<Pane size={65} minSize={10} class="text-sm text-primary">
		{#if scriptProgress}
			<JobProgressBar job={testJob} {scriptProgress} compact={true} />
		{/if}

		<OutputPickerInner
			{testJob}
			fullResult
			moduleId={mod.id}
			closeOnOutsideClick={true}
			getLogs
			{onUpdateMock}
			mock={mod.mock}
			isLoading={testIsLoading || loadingJob}
			path={`path` in mod.value ? mod.value.path : ''}
			{loopStatus}
			{disableMock}
			{disableHistory}
			bind:this={outputPickerInner}
		>
			{#snippet copilot_fix()}
				{#if lang && editor && diffEditor && stepsInputArgs.getStepArgs(mod.id) && selectedJob?.type === 'CompletedJob' && !selectedJob.success && getStringError(selectedJob.result)}
					<ScriptFix {lang} />
				{/if}
			{/snippet}
		</OutputPickerInner>
	</Pane>
	<Pane size={35} minSize={10}>
		{#if (mod.mock?.enabled && preview !== 'job' && testJob?.type !== 'QueuedJob') || preview === 'mock'}
			<LogViewer
				small
				content={undefined}
				isLoading={false}
				tag={undefined}
				customEmptyMessage="Using pinned data"
				{tagLabel}
			/>
		{:else if mod.value.type === 'aiagent' && logJob?.type === 'CompletedJob'}
			<AiAgentLogViewer
				tools={mod.value.tools}
				agentJob={{
					...logJob,
					type: 'CompletedJob'
				}}
				workspaceId={logJob.workspace_id}
			/>
		{:else}
			<LogViewer
				small
				jobId={logJob?.id}
				duration={logJob?.['duration_ms']}
				mem={logJob?.['mem_peak']}
				content={logJob?.['logs']}
				isLoading={(testIsLoading && logJob?.['running'] == false) || loadingJob}
				tag={logJob?.['tag']}
				{tagLabel}
			/>
		{/if}
	</Pane>
</Splitpanes>
