<script lang="ts">
	import { ScriptService, type FlowModule, type Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import JobLoader, { type Callbacks } from './JobLoader.svelte'
	import { getStepHistoryLoaderContext } from './stepHistoryLoader.svelte'

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

	const { flowStore, flowStateStore, pathStore, testSteps, previewArgs, modulesTestStates } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let jobLoader: JobLoader | undefined = $state(undefined)
	let jobProgressReset: () => void = () => {}
	let stepHistoryLoader = getStepHistoryLoaderContext()

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

		if (modulesTestStates.states[mod.id]) {
			modulesTestStates.states[mod.id].cancel = async () => {
				await jobLoader?.cancelJob()
				modulesTestStates.states[mod.id].testJob = undefined
			}
			modulesTestStates.runTestCb?.(mod.id)
		}

		const val = mod.value
		// let jobId: string | undefined = undefined
		let callbacks: Callbacks = {
			done: (x) => {
				jobDone(x)
				g
			}
		}
		if (val.type == 'rawscript') {
			await jobLoader?.runPreview(
				val.path ?? ($pathStore ?? '') + '/' + mod.id,
				val.content,
				val.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				flowStore?.val?.tag ?? val.tag,
				undefined,
				undefined,
				callbacks
			)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash: val.hash })
				: await getScriptByPath(val.path)
			await jobLoader?.runPreview(
				val.path,
				script.content,
				script.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				flowStore?.val?.tag ?? (val.tag_override ? val.tag_override : script.tag),
				script.lock,
				val.hash ?? script.hash,
				callbacks
			)
		} else if (val.type == 'flow') {
			await jobLoader?.runFlowByPath(val.path, args, callbacks)
		} else {
			throw Error('Not supported module type')
		}
	}

	function jobDone(testJob: Job & { result?: any }) {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob') {
			if ($flowStateStore[mod.id]) {
				$flowStateStore[mod.id].previewResult = testJob.result
				$flowStateStore[mod.id].previewSuccess = testJob.success
				$flowStateStore[mod.id].previewJobId = testJob.id
				$flowStateStore[mod.id].previewWorkspaceId = testJob.workspace_id
				$flowStateStore = $flowStateStore
			}
			stepHistoryLoader?.resetInitial(mod.id)
		}
		modulesTestStates.states[mod.id].testJob = undefined
	}

	export function cancelJob() {
		modulesTestStates.states[mod.id]?.cancel?.()
	}

	$effect(() => {
		// Update testIsLoading to read the state from parent components
		testIsLoading = modulesTestStates.states[mod.id]?.loading ?? false
	})

	$effect(() => {
		// Update testJob to read the state from parent components
		testJob = modulesTestStates.states[mod.id]?.testJob
	})

	modulesTestStates.states[mod.id] = {
		...(modulesTestStates.states[mod.id] ?? { loading: false }),
		loading: testIsLoading,
		testJob: testJob
	}
</script>

<JobLoader
	noCode={true}
	toastError={noEditor}
	bind:scriptProgress
	bind:this={jobLoader}
	bind:isLoading={
		() => modulesTestStates.states[mod.id]?.loading ?? false,
		(v) => {
			let newLoading = v ?? false
			if (modulesTestStates.states[mod.id]?.loading !== newLoading) {
				modulesTestStates.states[mod.id] = {
					...(modulesTestStates.states[mod.id] ?? {}),
					loading: newLoading
				}
			}
		}
	}
	bind:job={modulesTestStates.states[mod.id].testJob}
/>
