<script lang="ts">
	import type { Schema } from '$lib/common'
	import { ScriptService, type FlowModule, type Job, Script, JobService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getModifierKey } from '$lib/utils'
	import { getScriptByPath } from '$lib/scripts'

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
	import type DiffEditor from './DiffEditor.svelte'
	import type Editor from './Editor.svelte'
	import ScriptFix from './copilot/ScriptFix.svelte'

	export let mod: FlowModule
	export let schema: Schema
	export let pickableProperties: PickableProperties | undefined
	export let lang: Script.language
	export let editor: Editor
	export let diffEditor: DiffEditor

	const { flowStore, flowStateStore, testStepStore, pathStore } =
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
			await testJobLoader?.runPreview(
				val.path ?? ($pathStore ?? '') + '/' + mod.id,
				val.content,
				val.language,
				args,
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
				$flowStore?.tag ?? script.tag
			)
		} else if (val.type == 'flow') {
			await testJobLoader?.abstractRun(() =>
				JobService.runFlowByPath({ workspace: $workspaceStore!, path: val.path, requestBody: args })
			)
		} else {
			throw Error('Not supported module type')
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

		<div class="w-full justify-center flex">
			{#if testIsLoading}
				<Button size="sm" on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red">
					<Loader2 size={16} class="animate-spin mr-1" />
					Cancel
				</Button>
			{:else}
				<Button color="dark" btnClasses="truncate" size="sm" on:click={() => runTest(stepArgs)}
					>Run&nbsp; <Kbd small isModifier>{getModifierKey()}</Kbd>
					<Kbd small><span class="text-lg font-bold">⏎</span></Kbd></Button
				>
			{/if}
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
					isLoading={testIsLoading}
					tag={testJob?.tag}
				/>
			</Pane>
			<Pane size={50} minSize={10} class="text-sm text-tertiary">
				{#if testJob != undefined && 'result' in testJob && testJob.result != undefined}
					<pre class="overflow-x-auto break-words relative h-full px-2"
						><DisplayResult
							workspaceId={testJob?.workspace_id}
							jobId={testJob?.id}
							result={testJob.result}>
							<svelte:fragment slot="copilot-fix">
								{#if lang && editor && diffEditor && stepArgs && testJob?.result?.error}
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
