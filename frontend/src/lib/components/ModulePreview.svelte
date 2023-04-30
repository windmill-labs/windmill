<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ScriptService, type FlowModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getModifierKey, getScriptByPath } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Button from './common/button/Button.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import type { FlowEditorContext } from './flows/types'
	import LogViewer from './LogViewer.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import ModulePreviewForm from './ModulePreviewForm.svelte'
	import { Kbd } from './common'
	import { evalValue } from './flows/utils'
	import type { PickableProperties } from './flows/previousResults'

	export let mod: FlowModule
	export let schema: Schema
	export let pickableProperties: PickableProperties | undefined

	const { flowStore, flowStateStore, testStepStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	// Test
	let testJobLoader: TestJobLoader
	let testIsLoading = false
	let testJob: Job | undefined = undefined

	let stepArgs: Record<string, any> | undefined = Object.fromEntries(
		Object.keys(schema.properties).map((k) => [
			k,
			evalValue(k, mod, $testStepStore, pickableProperties, false)
		])
	)

	$: $testStepStore[mod.id] = stepArgs

	export function runTestWithStepArgs() {
		runTest(stepArgs)
	}

	export async function runTest(args: any) {
		const val = mod.value
		// let jobId: string | undefined = undefined
		if (val.type == 'rawscript') {
			await testJobLoader?.runPreview(val.path, val.content, val.language, args, val.tag)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
				: await getScriptByPath(val.path)
			await testJobLoader?.runPreview(val.path, script.content, script.language, args, script.tag)
		} else {
			throw Error('not testable module type')
		}
	}

	function jobDone() {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob' && `result` in testJob) {
			if ($flowStateStore[mod.id]) {
				$flowStateStore[mod.id].previewResult = testJob.result
				$flowStateStore = $flowStateStore
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

		{#if testIsLoading}
			<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="sm">
				<Loader2 size={16} class="animate-spin mr-1" />
				Cancel
			</Button>
		{:else}
			<Button btnClasses="w-full truncate" size="sm" on:click={() => runTest(stepArgs)}
				>Run&nbsp;<Kbd small>{getModifierKey()}</Kbd><Kbd small>Enter</Kbd></Button
			>
		{/if}

		<ModulePreviewForm {pickableProperties} {mod} {schema} bind:args={stepArgs} />
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
