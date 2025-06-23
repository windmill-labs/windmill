<script lang="ts" module>
	type testModuleState = {
		loading: boolean
		instances: number
		cancel?: () => void
	}

	let testModulesState = $state<Record<string, testModuleState>>({})
</script>

<script lang="ts">
	import { ScriptService, type FlowModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'
	import { getContext, onMount } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import TestJobLoader from './TestJobLoader.svelte'

	interface Props {
		mod: FlowModule
		testJob?: Job | undefined
		testIsLoading?: boolean
		noEditor?: boolean
		scriptProgress?: any
	}

	let {
		mod,
		testJob = $bindable(undefined),
		testIsLoading = $bindable(false),
		noEditor = false,
		scriptProgress = $bindable(undefined)
	}: Props = $props()

	const { flowStore, flowStateStore, pathStore, testSteps, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let testJobLoader: TestJobLoader | undefined = $state(undefined)
	let jobProgressReset: () => void = () => {}

	export function runTestWithStepArgs() {
		runTest(testSteps.getStepArgs(mod.id)?.value)
	}

	export function loadArgsAndRunTest() {
		testSteps?.updateStepArgs(mod.id, $flowStateStore, flowStore?.val, previewArgs?.val)
		runTest(testSteps.getStepArgs(mod.id)?.value)
	}

	export async function runTest(args: any) {
		// Not defined if JobProgressBar not loaded
		if (jobProgressReset) jobProgressReset()

		testModulesState[mod.id].cancel = testJobLoader?.cancelJob

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
				$flowStateStore[mod.id].initial = false
				$flowStateStore = $flowStateStore
			}
		}
		testJob = undefined
	}

	export function cancelJob() {
		testModulesState[mod.id]?.cancel?.()
	}

	$effect(() => {
		testIsLoading = testModulesState[mod.id]?.loading ?? false
	})

	onMount(() => {
		testModulesState[mod.id] = {
			...(testModulesState[mod.id] ?? { loading: false, instances: 0 }),
			loading: testIsLoading,
			instances: testModulesState[mod.id]!.instances + 1
		}
		return () => {
			testModulesState[mod.id]!.instances -= 1
			if (testModulesState[mod.id]!.instances < 1) {
				delete testModulesState[mod.id]
			}
		}
	})
</script>

<TestJobLoader
	toastError={noEditor}
	on:done={() => jobDone()}
	bind:scriptProgress
	bind:this={testJobLoader}
	bind:isLoading={
		() => testModulesState[mod.id]?.loading ?? false,
		(v) =>
			(testModulesState[mod.id] = {
				...testModulesState[mod.id],
				loading: v ?? false,
				instances: testModulesState[mod.id]?.instances ?? 0
			})
	}
	bind:job={testJob}
/>
