<script lang="ts">
	import {
		ScriptService,
		type AiAgent,
		type FlowModule,
		type JavascriptTransform,
		type Job
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import JobLoader, { type Callbacks } from './JobLoader.svelte'
	import { getStepHistoryLoaderContext } from './stepHistoryLoader.svelte'
	import { loadSchemaFromModule } from './flows/flowInfers'

	interface Props {
		mod: FlowModule
		testJob?: Job | undefined
		testIsLoading?: boolean
		noEditor?: boolean
		scriptProgress?: any
		onJobDone?: () => void
	}

	let {
		mod,
		testJob = $bindable(undefined),
		testIsLoading = $bindable(false),
		noEditor = false,
		scriptProgress = $bindable(undefined),
		onJobDone
	}: Props = $props()

	const { flowStore, flowStateStore, pathStore, stepsInputArgs, previewArgs, modulesTestStates } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let jobLoader: JobLoader | undefined = $state(undefined)
	let jobProgressReset: () => void = () => {}
	let stepHistoryLoader = getStepHistoryLoaderContext()

	export function runTestWithStepArgs() {
		const args = stepsInputArgs.getStepArgs(mod.id)
		runTest(args)
	}

	export function loadArgsAndRunTest() {
		stepsInputArgs?.updateStepArgs(mod.id, flowStateStore.val, flowStore?.val, previewArgs?.val)
		runTestWithStepArgs()
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
				callbacks,
				$pathStore
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
				callbacks,
				$pathStore
			)
		} else if (val.type == 'flow') {
			await jobLoader?.runFlowByPath(val.path, args, callbacks)
		} else if (val.type == 'aiagent') {
			const { schema } = await loadSchemaFromModule(mod)

			const inputTransforms: { [key: string]: JavascriptTransform } = Object.fromEntries(
				Object.keys(args).map((key) => [
					key,
					{
						expr: `flow_input.${key}`,
						type: 'javascript'
					}
				])
			)

			await jobLoader?.runFlowPreview(
				args,
				{
					value: {
						modules: [
							{
								id: mod.id,
								value: {
									type: 'aiagent',
									tools: mod.value.type == 'aiagent' ? mod.value.tools : [],
									input_transforms: inputTransforms as AiAgent['input_transforms']
								}
							}
						]
					},
					summary: '',
					schema
				},
				callbacks,
				$pathStore
			)
		} else {
			throw Error('Not supported module type')
		}
	}

	function jobDone(testJob: Job & { result?: any }) {
		if (testJob && !testJob.canceled && testJob.type == 'CompletedJob') {
			if (flowStateStore.val[mod.id]) {
				flowStateStore.val[mod.id] = {
					...flowStateStore.val[mod.id],
					previewResult: testJob.result,
					previewSuccess: testJob.success,
					previewJobId: testJob.id
				}
			}
			stepHistoryLoader?.resetInitial(mod.id)
		}
		if (modulesTestStates.states[mod.id]) {
			modulesTestStates.states[mod.id].testJob = testJob
		}
		onJobDone?.()
	}

	export function cancelJob() {
		modulesTestStates.states[mod.id]?.cancel?.()
	}

	$effect(() => {
		// Update testIsLoading to read the state from parent components
		testIsLoading = modulesTestStates.states?.[mod.id]?.loading ?? false
	})

	$effect(() => {
		// Update testJob to read the state from parent components
		testJob = modulesTestStates.states?.[mod.id]?.testJob
	})

	modulesTestStates.states[mod.id] = {
		...(modulesTestStates.states?.[mod.id] ?? { loading: false }),
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
			if (modulesTestStates.states && modulesTestStates.states?.[mod.id]?.loading !== newLoading) {
				modulesTestStates.states[mod.id] = {
					...(modulesTestStates.states?.[mod.id] ?? {}),
					loading: newLoading,
					hiddenInGraph: false
				}
			}
		}
	}
	bind:job={
		() => modulesTestStates.states[mod.id]?.testJob,
		(v) => modulesTestStates.states[mod.id] && (modulesTestStates.states[mod.id].testJob = v)
	}
	loadPlaceholderJobOnStart={{
		type: 'QueuedJob',
		id: '',
		running: false,
		canceled: false,
		job_kind: 'preview',
		permissioned_as: '',
		is_flow_step: false,
		email: '',
		visible_to_owner: true,
		tag: ''
	}}
/>
