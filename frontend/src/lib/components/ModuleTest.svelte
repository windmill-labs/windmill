<script lang="ts">
	import { ScriptService, type FlowModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import TestJobLoader from './TestJobLoader.svelte'

	interface Props {
		mod: FlowModule
		testJob?: Job | undefined
		testIsLoading?: boolean
		noEditor?: boolean
		scriptProgress?: any
		onJobDone?: ((job: Job | undefined) => void) | undefined
	}

	let {
		mod,
		testJob = $bindable(undefined),
		testIsLoading = $bindable(false),
		noEditor = false,
		scriptProgress = $bindable(undefined),
		onJobDone = undefined
	}: Props = $props()

	const { flowStore, flowStateStore, pathStore, testSteps, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let testJobLoader: TestJobLoader | undefined = $state(undefined)
	let jobProgressReset: () => void = () => {}

	export function runTestWithStepArgs() {
		runTest(testSteps.getStepArgs(mod.id))
	}

	export function loadArgsAndRunTest() {
		testSteps?.updateStepArgs(mod.id, $flowStateStore, flowStore?.val, previewArgs?.val)
		runTest(testSteps.getStepArgs(mod.id))
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
				flowStore?.val?.tag ?? val.tag
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
				flowStore?.val?.tag ?? (val.tag_override ? val.tag_override : script.tag),
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
		const finishedJob = testJob
		testJob = undefined
		onJobDone?.(finishedJob)
	}

	export function cancelJob() {
		testJobLoader?.cancelJob()
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
