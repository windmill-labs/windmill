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

	export let lang: Script['language']
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined
	export let loopStatus:
		| { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' }
		| undefined = undefined
	export let lastJob: Job | undefined = undefined
	export let scriptProgress: number | undefined = undefined
	export let testJob: Job | undefined = undefined
	export let mod: FlowModule
	export let testIsLoading: boolean = false
	export let disableMock: boolean = false
	export let disableHistory: boolean = false

	const { flowStore, testSteps } = getContext<FlowEditorContext>('FlowEditorContext')

	let selectedJob: Job | undefined = undefined
	let fetchingLastJob = false
	let preview: 'mock' | 'job' | undefined = undefined
	let outputPicker: OutputPickerInner | undefined = undefined
	let jobProgressReset: () => void = () => {}

	$: lastJob && outputPicker?.setLastJob(lastJob, false)
	$: testJob && outputPicker?.setLastJob(testJob, true)

	let forceJson = false
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
			bind:this={outputPicker}
			fullResult
			moduleId={mod.id}
			closeOnOutsideClick={true}
			getLogs
			on:updateMock={({ detail }) => {
				mod.mock = detail
				$flowStore = $flowStore
			}}
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
			<svelte:fragment slot="copilot-fix">
				{#if lang && editor && diffEditor && testSteps.getStepArgs(mod.id) && selectedJob?.type === 'CompletedJob' && !selectedJob.success && getStringError(selectedJob.result)}
					<ScriptFix {lang} />
				{/if}
			</svelte:fragment>
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
