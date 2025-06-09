<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ScriptService, type FlowModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'

	import { CornerDownLeft, Loader2 } from 'lucide-svelte'
	import { getContext } from 'svelte'

	import Button from './common/button/Button.svelte'
	import type { FlowEditorContext } from './flows/types'

	import TestJobLoader from './TestJobLoader.svelte'
	import ModulePreviewForm from './ModulePreviewForm.svelte'

	import { evalValue } from './flows/utils'
	import type { PickableProperties } from './flows/previousResults'

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
				flowStore.val?.tag ?? val.tag
			)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
				: await getScriptByPath(val.path)
			await testJobLoader?.runPreview(
				val.path,
				script.content,
				script.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				flowStore.val?.tag ?? (val.tag_override ? val.tag_override : script.tag),
				script.lock,
				val.hash ?? script.hash
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
	{#if flowStore.val.value.same_worker}
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
</div>
