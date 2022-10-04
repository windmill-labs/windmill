<script lang="ts">
	import type { Schema } from '$lib/common'
	import type { FlowModule, Job } from '$lib/gen'
	import { getScriptByPath, sendUserToast, truncateRev } from '$lib/utils'

	import { HSplitPane, VSplitPane } from 'svelte-split-pane'

	import RunForm from './RunForm.svelte'
	import TestJobLoader from './TestJobLoader.svelte'
	import LogViewer from './LogViewer.svelte'
	import DisplayResult from './DisplayResult.svelte'
	import Button from './common/button/Button.svelte'
	import { faRotateRight } from '@fortawesome/free-solid-svg-icons'
	import { flowStateStore } from './flows/flowState'

	let testJobLoader: TestJobLoader

	// Test
	let testIsLoading = false
	let testJob: Job | undefined

	export let mod: FlowModule
	export let schema: Schema
	export let indices: [number, number | undefined]

	let stepArgs: Record<string, any> = {}

	export function runTestWithStepArgs() {
		runTest(stepArgs)
	}

	export async function runTest(args: any) {
		const val = mod.value
		if (val.type == 'rawscript') {
			await testJobLoader?.runPreview(val.path, val.content, val.language, args)
		} else if (val.type == 'script') {
			const script = await getScriptByPath(val.path)
			await testJobLoader?.runPreview(val.path, script.content, script.language, args)
		} else {
			throw Error('not testable module type')
		}
		sendUserToast(`started test ${truncateRev(testJob?.id ?? '', 10)}`)
	}

	function jobDone() {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob' && `result` in testJob) {
			const result = testJob.result
			const pMod = $flowStateStore.modules[indices[0]]
			if (pMod) {
				if (indices[1] != undefined && pMod.childFlowModules) {
					const cMod = pMod.childFlowModules[indices[1]]
					if (cMod) {
						cMod.previewResult = result
					}
				} else {
					pMod.previewResult = result
				}
				$flowStateStore.modules[indices[0]] = pMod
			}
		}
	}
</script>

<TestJobLoader
	on:done={jobDone}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>
<HSplitPane leftPaneSize="50%" rightPaneSize="50%" minLeftPaneSize="20%" minRightPaneSize="20%">
	<left slot="left" class="relative">
		<div class="overflow-auto h-full p-4">
			<RunForm
				runnable={{ summary: mod.summary ?? '', schema, description: '' }}
				runAction={(_, args) => runTest(args)}
				schedulable={false}
				buttonText="Test just this step (Ctrl+Enter)"
				detailed={false}
				bind:args={stepArgs}
			/>
			{#if testIsLoading}
				<Button
					on:click={testJobLoader?.cancelJob}
					btnClasses="w-full mt-4"
					color="red"
					size="sm"
					startIcon={{
						icon: faRotateRight,
						classes: 'animate-spin'
					}}
				>
					Cancel
				</Button>
			{/if}
		</div>
	</left>
	<right slot="right">
		<div class="overflow-auto h-full">
			<VSplitPane topPanelSize="50%" downPanelSize="50%">
				<top slot="top">
					<LogViewer content={testJob?.logs} isLoading={testIsLoading} />
				</top>
				<down slot="down">
					<pre class="overflow-x-auto break-all relative h-full p-2 text-sm"
						>{#if testJob && 'result' in testJob && testJob.result != undefined}<DisplayResult
								result={testJob.result}
							/>
						{:else if testIsLoading}Waiting for Result...
						{:else}Test to see the result here
						{/if}
        	</pre>
				</down>
			</VSplitPane>
		</div>
	</right>
</HSplitPane>
