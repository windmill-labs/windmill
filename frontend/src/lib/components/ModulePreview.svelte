<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ScriptService, type FlowModule, type Job, type Script, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'

	import { CornerDownLeft, Loader2 } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Button from './common/button/Button.svelte'
	import type { FlowEditorContext } from './flows/types'
	import LogViewer from './LogViewer.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import ModulePreviewForm from './ModulePreviewForm.svelte'
	import JobProgressBar from '$lib/components/jobs/JobProgressBar.svelte'
	import { evalValue } from './flows/utils'
	import type { PickableProperties } from './flows/previousResults'
	import type DiffEditor from './DiffEditor.svelte'
	import type Editor from './Editor.svelte'
	import ScriptFix from './copilot/ScriptFix.svelte'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'

	export let mod: FlowModule
	export let schema: Schema | { properties?: Record<string, any> }
	export let pickableProperties: PickableProperties | undefined
	export let lang: Script['language']
	export let editor: Editor | undefined
	export let diffEditor: DiffEditor | undefined
	export let noEditor = false
	export let lastJob: Job | undefined = undefined
	export let loopStatus:
		| { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' }
		| undefined = undefined

	const { flowStore, flowStateStore, testStepStore, pathStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	// Test
	let scriptProgress = undefined
	let testJobLoader: TestJobLoader
	let testIsLoading = false
	let testJob: Job | undefined = undefined
	let selectedJob: Job | undefined = undefined
	let outputPicker: OutputPickerInner | undefined = undefined
	let jobProgressReset: () => void
	let fetchingLastJob = false
	let preview: 'mock' | 'job' | undefined = undefined

	let stepArgs: Record<string, any> | undefined = Object.fromEntries(
		Object.keys(schema.properties ?? {}).map((k) => [
			k,
			evalValue(k, mod, $testStepStore, pickableProperties, false)
		])
	)

	$: $testStepStore[mod.id] = stepArgs

	export function runTestWithStepArgs() {
		runTest(stepArgs)
	}

	export async function runTest(args: any) {
		// Not defined if JobProgressBar not loaded
		if (jobProgressReset) jobProgressReset()

		const val = mod.value
		// let jobId: string | undefined = undefined
		if (val.type == 'rawscript') {
			await testJobLoader?.runPreview(
				val.path ?? ($pathStore ?? '') + '/' + mod.id,
				val.content,
				val.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				$flowStore?.tag ?? val.tag
			)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
				: await getScriptByPath(val.path)
			await testJobLoader?.runPreview(
				val.path,
				script.content,
				script.language,
				args,
				$flowStore?.tag ?? (val.tag_override ? val.tag_override : script.tag)
			)
		} else if (val.type == 'flow') {
			await testJobLoader?.abstractRun(() =>
				JobService.runFlowByPath({
					workspace: $workspaceStore!,
					path: val.path,
					requestBody: args,
					skipPreprocessor: true
				})
			)
		} else {
			throw Error('Not supported module type')
		}
	}

	function jobDone() {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob' && `result` in testJob) {
			if ($flowStateStore[mod.id]) {
				$flowStateStore[mod.id].previewResult = testJob.result
				$flowStateStore[mod.id].previewSuccess = testJob.success
				$flowStateStore[mod.id].previewJobId = testJob.id
				$flowStateStore[mod.id].previewWorkspaceId = testJob.workspace_id
				$flowStateStore = $flowStateStore
			}
		}
	}

	$: testJob && outputPicker?.setLastJob(testJob, true)
	$: lastJob && outputPicker?.setLastJob(lastJob, false)

	let forceJson = false
</script>

<TestJobLoader
	toastError={noEditor}
	on:done={() => jobDone()}
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>
<Splitpanes>
	<Pane size={50} minSize={20} class="p-4">
		{#if $flowStore.value.same_worker}
			<div class="mb-1 bg-yellow-100 text-yellow-700 p-1 text-xs"
				>The `./shared` folder is not passed across individual "Test this step"</div
			>
		{/if}

		<div class="w-full justify-center flex">
			{#if testIsLoading}
				<Button size="sm" on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red">
					<Loader2 size={16} class="animate-spin mr-1" />
					Cancel
				</Button>
			{:else}
				<Button
					color="dark"
					btnClasses="truncate"
					size="sm"
					on:click={() => runTest(stepArgs)}
					shortCut={{
						Icon: CornerDownLeft
					}}
				>
					Run
				</Button>
			{/if}
		</div>

		<ModulePreviewForm {pickableProperties} {mod} {schema} bind:args={stepArgs} />
	</Pane>
	<Pane size={50} minSize={20}>
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
				>
					<svelte:fragment slot="copilot-fix">
						{#if lang && editor && diffEditor && stepArgs && selectedJob && 'result' in selectedJob && selectedJob.result && typeof selectedJob.result == 'object' && `error` in selectedJob.result && selectedJob.result.error}
							<ScriptFix
								error={JSON.stringify(selectedJob.result.error)}
								{lang}
								{editor}
								{diffEditor}
								args={stepArgs}
							/>
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
	</Pane>
</Splitpanes>
