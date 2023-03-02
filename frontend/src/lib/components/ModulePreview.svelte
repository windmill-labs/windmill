<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ScriptService, type FlowModule, type InputTransform, type Job } from '$lib/gen'
	import { getScriptByPath, sendUserToast, truncateRev } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import RunForm from './RunForm.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import LogViewer from './LogViewer.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import Button from './common/button/Button.svelte'
	import { flowStateStore, testStepStore } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import { workspaceStore } from '$lib/stores'
	import { Loader2 } from 'lucide-svelte'

	export let mod: FlowModule
	export let schema: Schema

	// Test
	let testJobLoader: TestJobLoader
	let testIsLoading = false
	let testJob: Job | undefined = undefined

	let stepArgs: Record<string, any> | undefined =
		$testStepStore[mod.id] ??
		Object.entries(mod.value['input_transforms'] ?? {}).reduce((acc, [k, v]) => {
			let t = v as InputTransform
			if (t.type == 'static') {
				acc[k] = t.value
				return acc
			}
			return acc
		}, {})

	$: $testStepStore[mod.id] = stepArgs

	export function runTestWithStepArgs() {
		runTest(stepArgs)
	}

	export async function runTest(args: any) {
		const val = mod.value
		let jobId: string | undefined = undefined
		if (val.type == 'rawscript') {
			jobId = await testJobLoader?.runPreview(val.path, val.content, val.language, args)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
				: await getScriptByPath(val.path)
			jobId = await testJobLoader?.runPreview(val.path, script.content, script.language, args)
		} else {
			throw Error('not testable module type')
		}
	}

	function jobDone() {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob' && `result` in testJob) {
			if ($flowStateStore[mod.id]?.previewResult) {
				$flowStateStore[mod.id].previewResult = testJob.result
			}
		}
	}
</script>

<TestJobLoader
	on:done={() => jobDone()}
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

		<RunForm
			noVariablePicker
			loading={testIsLoading}
			runnable={{ summary: mod.summary ?? '', schema, description: '' }}
			runAction={(_, args) => runTest(args)}
			schedulable={false}
			buttonText="Test (Ctrl+Enter)"
			detailed={false}
			topButton
			bind:args={stepArgs}
			isFlow={false}
		/>
		{#if testIsLoading}
			<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full mt-4" color="red" size="sm">
				<Loader2 class="animate-spin mr-1" />
				Cancel
			</Button>
		{/if}
	</Pane>
	<Pane size={50} minSize={20}>
		<Splitpanes horizontal>
			<Pane size={50} minSize={10}>
				<LogViewer content={testJob?.logs} isLoading={testIsLoading} />
			</Pane>
			<Pane size={50} minSize={10} class="text-sm text-gray-600">
				{#if testJob != undefined && 'result' in testJob && testJob.result != undefined}
					<pre class="overflow-x-auto break-words relative h-full px-2">
						<DisplayResult result={testJob.result} />
					</pre>
				{:else}
					<div class="p-2">
						{#if testIsLoading}
							<Loader2 class="animate-spin" />
						{:else}
							Test to see the result here
						{/if}
					</div>
				{/if}
			</Pane>
		</Splitpanes>
	</Pane>
</Splitpanes>
