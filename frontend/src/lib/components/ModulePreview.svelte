<script lang="ts">
	import type { Schema } from '$lib/common'
<<<<<<< HEAD
	import { ScriptService, type FlowModule, type Job, type Script } from '$lib/gen'
=======
	import { ScriptService, type FlowModule, type Job, JobService } from '$lib/gen'
>>>>>>> main
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'

	import { Loader2 } from 'lucide-svelte'
	import { getContext } from 'svelte'
<<<<<<< HEAD
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DisplayResult from './DisplayResult.svelte'
=======
	import Button from './common/button/Button.svelte'
>>>>>>> main
	import type { FlowEditorContext } from './flows/types'

	import TestJobLoader from './TestJobLoader.svelte'
	import ModulePreviewForm from './ModulePreviewForm.svelte'

	import { evalValue } from './flows/utils'
	import type { PickableProperties } from './flows/previousResults'
<<<<<<< HEAD
	import type DiffEditor from './DiffEditor.svelte'
	import type Editor from './Editor.svelte'
	import ScriptFix from './copilot/ScriptFix.svelte'
	import RunButton from '$lib/components/RunButton.svelte'
=======
>>>>>>> main

	export let mod: FlowModule
	export let schema: Schema | { properties?: Record<string, any> }
	export let pickableProperties: PickableProperties | undefined
	export let testJob: Job | undefined = undefined
	export let testIsLoading = false
	export let noEditor = false
	export let scriptProgress = undefined

	const { flowStore, flowStateStore, testStepStore, pathStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	// Test

	let testJobLoader: TestJobLoader

	let jobProgressReset: () => void = () => {}

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
			await testJobLoader?.runFlowByPath(val.path, args)
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
		testJob = undefined
	}
</script>

<TestJobLoader
	toastError={noEditor}
	on:done={() => jobDone()}
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<div class="p-4">
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

<<<<<<< HEAD
		<div class="w-full justify-center flex">
			<RunButton
				isLoading={testIsLoading}
				hideShortcut={false}
				onRun={() => runTest(stepArgs)}
				onCancel={() => testJobLoader?.cancelJob()}
			/>
		</div>

		<ModulePreviewForm {pickableProperties} {mod} {schema} bind:args={stepArgs} />
	</Pane>
	<Pane size={50} minSize={20}>
		<Splitpanes horizontal>
			<Pane size={50} minSize={10}>
				<LogViewer
					small
					jobId={testJob?.id}
					duration={testJob?.['duration_ms']}
					mem={testJob?.['mem_peak']}
					content={testJob?.logs}
					isLoading={testIsLoading && testJob?.['running'] == false}
					tag={testJob?.tag}
				/>
			</Pane>
			<Pane size={50} minSize={10} class="text-sm text-tertiary">
				{#if scriptProgress}
					<JobProgressBar
						job={testJob}
						bind:scriptProgress
						bind:reset={jobProgressReset}
						compact={true}
					/>
				{/if}
				{#if testJob != undefined && 'result' in testJob && testJob.result != undefined}
					<div class="break-words relative h-full p-2">
						<DisplayResult
							bind:forceJson
							workspaceId={testJob?.workspace_id}
							jobId={testJob?.id}
							result={testJob.result}
						>
							<svelte:fragment slot="copilot-fix">
								{#if lang && editor && diffEditor && stepArgs && typeof testJob?.result == 'object' && `error` in testJob?.result && testJob?.result.error}
									<ScriptFix
										error={JSON.stringify(testJob.result.error)}
										{lang}
										{editor}
										{diffEditor}
										args={stepArgs}
									/>
								{/if}
							</svelte:fragment>
						</DisplayResult>
					</div>
				{:else}
					<div class="p-2">
						{#if testIsLoading}
							{#if !scriptProgress}
								<Loader2 class="animate-spin" />
							{/if}
						{:else}
							Test to see the result here
						{/if}
					</div>
				{/if}
			</Pane>
		</Splitpanes>
	</Pane>
</Splitpanes>
=======
	<ModulePreviewForm {pickableProperties} {mod} {schema} bind:args={stepArgs} />
</div>
>>>>>>> main
