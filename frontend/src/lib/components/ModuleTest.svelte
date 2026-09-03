<script lang="ts">
	import {
		ScriptService,
		type AiAgent,
		type FlowModule,
		type InputTransform,
		type JavascriptTransform,
		type Job
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { getScriptByPath } from '$lib/scripts'
	import { getContext, untrack } from 'svelte'
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

	const {
		flowStore,
		flowStateStore,
		pathStore,
		stepsInputArgs,
		previewArgs,
		modulesTestStates,
		devTempScriptRefs,
		opWorkspace
	} = getContext<FlowEditorContext>('FlowEditorContext')

	let previewBase = $derived($pathStore ?? '')

	// Acting workspace when the flow editor runs in an AI session; else the nav workspace.
	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

	let jobLoader: JobLoader | undefined = $state(undefined)
	let jobProgressReset: () => void = () => {}
	let stepHistoryLoader = getStepHistoryLoaderContext()

	// Every explicit run re-evaluates the args with errors surfaced. The reactive evaluations
	// that follow each flow edit stay quiet, so without this a failing expression is silently
	// `undefined` in what the run is built from. Manually edited args are preserved across the
	// refresh by `initializeFromSchema`.
	export function runTestWithStepArgs() {
		stepsInputArgs?.updateStepArgs(
			mod.id,
			flowStateStore.val,
			flowStore?.val,
			previewArgs?.val,
			true
		)
		runTest(stepsInputArgs.getStepArgs(mod.id))
	}

	// A step's timeout is an InputTransform. Only a static numeric value can be applied
	// to a single-step preview; dynamic expressions are evaluated server-side and only
	// take effect when running the full flow.
	function staticTimeout(timeout: FlowModule['timeout']): number | undefined {
		if (timeout?.type === 'static' && typeof timeout.value === 'number') {
			return timeout.value
		}
		return undefined
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
		const timeout = staticTimeout(mod.timeout)
		// let jobId: string | undefined = undefined
		let callbacks: Callbacks = {
			done: (x) => {
				jobDone(x)
			}
		}
		if (val.type == 'rawscript') {
			await jobLoader?.runPreview(
				// An empty base stays empty: `'' + '/' + id` is an absolute path, which
				// `require_path_read_access_for_preview` rejects outright. A flow with no path yet
				// previews unnamed instead.
				val.path ?? (previewBase ? previewBase + '/' + mod.id : ''),
				val.content,
				val.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				flowStore?.val?.tag ?? val.tag,
				undefined,
				undefined,
				callbacks,
				previewBase,
				undefined,
				devTempScriptRefs?.(),
				timeout
			)
		} else if (val.type == 'script') {
			const script = val.hash
				? await ScriptService.getScriptByHash({ workspace: opWs!, hash: val.hash })
				: await getScriptByPath(val.path, opWs)
			await jobLoader?.runPreview(
				val.path,
				script.content,
				script.language,
				mod.id === 'preprocessor' ? { _ENTRYPOINT_OVERRIDE: 'preprocessor', ...args } : args,
				flowStore?.val?.tag ?? (val.tag_override ? val.tag_override : script.tag),
				script.lock,
				val.hash ?? script.hash,
				callbacks,
				previewBase,
				undefined,
				undefined,
				timeout
			)
		} else if (val.type == 'flow') {
			await jobLoader?.runFlowByPath(val.path, args, callbacks)
		} else if (val.type == 'aiagent') {
			const { schema } = await loadSchemaFromModule(mod, opWs)

			const agentVal = val

			// The test form only covers the schema it was given, and for a standalone agent that may be
			// the flow-local one (the agent editor shows the brain in its own form, not here). Take the
			// brain from the module as authored and let the form's own keys win over it, so an edit made
			// in the form after the test panel mounted is what runs. A linked agent needs none of this:
			// the server reads its brain from the resource.
			const inputTransforms: { [key: string]: JavascriptTransform | InputTransform } = {
				...(agentVal.agent
					? {}
					: ((agentVal.input_transforms ?? {}) as Record<string, InputTransform>)),
				...Object.fromEntries(
					Object.keys(args).map((key) => [
						key,
						{
							expr: `flow_input.${key}`,
							type: 'javascript'
						}
					])
				)
			}

			await jobLoader?.runFlowPreview(
				args,
				{
					value: {
						modules: [
							{
								id: mod.id,
								// A linked step has no tools of its own: the resource's tools are resolved
								// server-side from `agent`. `tool_inputs` goes in either way — a step forked
								// for editing has no `agent` yet still carries the flow's bindings, which the
								// runtime overlays, so the preview must test against them too.
								value: {
									type: 'aiagent',
									...(agentVal.agent ? { agent: agentVal.agent } : { tools: agentVal.tools ?? [] }),
									tool_inputs: agentVal.tool_inputs,
									input_transforms: inputTransforms as AiAgent['input_transforms']
								} as Extract<FlowModule['value'], { type: 'aiagent' }>
							}
						]
					},
					summary: '',
					schema
				},
				callbacks,
				previewBase
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
					previewJobId: testJob.id,
					previewLogs: testJob['logs']
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

	modulesTestStates.states[untrack(() => mod).id] = {
		...(modulesTestStates.states?.[untrack(() => mod).id] ?? { loading: false }),
		loading: testIsLoading,
		testJob: testJob
	}
</script>

<JobLoader
	noCode={true}
	toastError={noEditor}
	workspaceOverride={opWs}
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
