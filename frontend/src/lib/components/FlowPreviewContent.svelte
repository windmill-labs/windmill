<script lang="ts">
	import {
		type Job,
		JobService,
		type RestartedFrom,
		type OpenFlow,
		type ScriptLang
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button } from './common'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import SchemaFormWithArgPicker from './SchemaFormWithArgPicker.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import FlowExecutionStatus from './runs/FlowExecutionStatus.svelte'
	import JobDetailHeader from './runs/JobDetailHeader.svelte'
	import {
		AlertTriangle,
		Circle,
		CornerDownLeft,
		Download,
		Loader2,
		Play,
		RefreshCw,
		X
	} from 'lucide-svelte'
	import { emptyString, sendUserToast, type StateStore } from '$lib/utils'
	import { dfs } from './flows/dfs'
	import { sliceModules } from './flows/flowStateUtils.svelte'
	import InputSelectedBadge from './schema/InputSelectedBadge.svelte'
	import Toggle from './Toggle.svelte'
	import JsonInputs from './JsonInputs.svelte'
	import FlowHistoryJobPicker from './FlowHistoryJobPicker.svelte'
	import type { DurationStatus, GraphModuleState } from './graph'
	import { getStepHistoryLoaderContext } from './stepHistoryLoader.svelte'
	import FlowChat from './flows/conversations/FlowChat.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'
	import FlowRestartButton from './FlowRestartButton.svelte'
	import { createFlowRecording, setActiveRecording } from './recording/flowRecording.svelte'
	import type { FlowRecording } from './recording/types'

	interface Props {
		previewMode: 'upTo' | 'whole'
		open: boolean
		preventEscape?: boolean
		jobId?: string | undefined
		job?: Job | undefined
		initial?: boolean
		selectedJobStep?: string | undefined
		selectedJobStepIsTopLevel?: boolean | undefined
		selectedJobStepType?: 'single' | 'forloop' | 'branchall'
		rightColumnSelect?: 'timeline' | 'node_status' | 'node_definition' | 'user_states'
		branchOrIterationN?: number
		scrollTop?: number
		localModuleStates?: Record<string, GraphModuleState>
		localDurationStatuses?: Record<string, DurationStatus>
		onRunPreview?: () => void
		render?: boolean
		onJobDone?: () => void
		upToId?: string | undefined
		customUi?: {
			tagLabel?: string | undefined
		}
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
	}

	let {
		previewMode = $bindable(),
		open,
		preventEscape = $bindable(false),
		jobId = $bindable(undefined),
		job = $bindable(undefined),
		initial = $bindable(false),
		selectedJobStep = $bindable(undefined),
		selectedJobStepIsTopLevel = $bindable(undefined),
		selectedJobStepType = $bindable('single'),
		rightColumnSelect = $bindable('timeline'),
		branchOrIterationN = $bindable(0),
		scrollTop = $bindable(0),
		localModuleStates = $bindable({}),
		localDurationStatuses = $bindable({}),
		onRunPreview,
		render = false,
		onJobDone,
		upToId = undefined,
		suspendStatus
	}: Props = $props()

	let restartBranchNames: [number, string][] = []
	let isRunning: boolean = $state(false)
	let jsonView: boolean = $state(false)
	let jsonEditor: JsonInputs | undefined = $state(undefined)
	let schemaHeight = $state(0)
	let isValid: boolean = $state(true)
	let isOwner: boolean = $state(false)

	export async function test(conversationId?: string): Promise<string | undefined> {
		renderCount++
		return await runPreview(previewArgs.val, undefined, conversationId)
	}

	const {
		selectionManager,
		previewArgs,
		flowStateStore,
		flowStore,
		pathStore,
		initialPathStore,
		fakeInitialPath,
		customUi,
		executionCount
	} = $state(getContext<FlowEditorContext>('FlowEditorContext'))
	const dispatch = createEventDispatcher()

	let renderCount: number = $state(0)
	let schemaFormWithArgPicker: SchemaFormWithArgPicker | undefined = $state(undefined)
	let currentJobId: string | undefined = $state(undefined)
	let stepHistoryLoader = getStepHistoryLoaderContext()
	let flowProgressBar: FlowProgressBar | undefined = $state(undefined)

	// Recording & replay
	let flowRecording = createFlowRecording()
	let lastRecording: FlowRecording | undefined = $state(undefined)
	let recordingMode: boolean = $state(false)

	export function setRecordingMode(on: boolean) {
		recordingMode = on
	}

	let loadingHistory = $state(false)

	let shouldUseStreaming = $derived.by(() => {
		const modules = flowStore.val.value?.modules
		const lastModule = modules && modules.length > 0 ? modules[modules.length - 1] : undefined
		return (
			lastModule?.value?.type === 'aiagent' &&
			lastModule?.value?.input_transforms?.streaming?.type === 'static' &&
			lastModule?.value?.input_transforms?.streaming?.value === true
		)
	})

	function extractFlow(previewMode: 'upTo' | 'whole'): OpenFlow {
		if (previewMode === 'whole') {
			return flowStore.val
		} else {
			const flow = stateSnapshot(flowStore).val as OpenFlow
			const idOrders = dfs(flow.value.modules, (x) => x.id)
			let upToIndex = idOrders.indexOf(upToId ?? selectionManager.getSelectedId() ?? '')

			if (upToIndex != -1) {
				flow.value.modules = sliceModules(flow.value.modules, upToIndex, idOrders)
			}
			return flow
		}
	}

	let lastPreviewFlow: undefined | string = $state(undefined)
	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined,
		conversationId?: string | undefined
	) {
		let newJobId: string | undefined = undefined
		if (stepHistoryLoader?.flowJobInitial !== false) {
			stepHistoryLoader?.setFlowJobInitial(false)
		}
		try {
			lastPreviewFlow = JSON.stringify(flowStore.val)
			flowProgressBar?.reset()
			const newFlow = extractFlow(previewMode)
			newJobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom, conversationId)
			jobId = newJobId
			isRunning = true
			if (inputSelected) {
				savedArgs = $state.snapshot(previewArgs.val)
				inputSelected = undefined
			}
			onRunPreview?.()
		} catch (e) {
			sendUserToast('Could not run preview', true, undefined, e.toString())
			isRunning = false
			jobId = undefined
		}
		schemaFormWithArgPicker?.refreshHistory()
		return newJobId
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Enter':
					if (event.ctrlKey || event.metaKey) {
						event.preventDefault()
						runPreview(previewArgs.val, undefined)
					}
					break

				case 'Escape':
					if (preventEscape) {
						selectInput(undefined)
						event.preventDefault()
						event.stopPropagation()
					}
					break
			}
		}
	}

	function onSelectedJobStepChange() {
		if (selectedJobStep !== undefined && job?.flow_status?.modules !== undefined) {
			selectedJobStepIsTopLevel =
				job?.flow_status?.modules.map((m) => m.id).indexOf(selectedJobStep) >= 0
			let moduleDefinition = job?.raw_flow?.modules.find((m) => m.id == selectedJobStep)
			if (moduleDefinition?.value.type == 'forloopflow') {
				selectedJobStepType = 'forloop'
			} else if (moduleDefinition?.value.type == 'branchall') {
				selectedJobStepType = 'branchall'
				moduleDefinition?.value.branches.forEach((branch, idx) => {
					restartBranchNames.push([
						idx,
						emptyString(branch.summary) ? `Branch #${idx}` : branch.summary!
					])
				})
			} else {
				selectedJobStepType = 'single'
			}
		}
	}

	let savedArgs = $state(previewArgs.val)
	let inputSelected: 'captures' | 'history' | 'saved' | undefined = $state(undefined)
	async function selectInput(input, type?: 'captures' | 'history' | 'saved' | undefined) {
		if (!input) {
			previewArgs.val = savedArgs
			inputSelected = undefined
			setTimeout(() => {
				preventEscape = false
			}, 100)
		} else {
			previewArgs.val = input
			inputSelected = type
			preventEscape = true
			jsonEditor?.setCode(JSON.stringify(previewArgs.val ?? {}, null, '\t'))
		}
	}

	export function refresh() {
		renderCount++
	}

	async function recordAndTest() {
		lastRecording = undefined
		flowRecording.start($pathStore)
		flowRecording.setFlow(extractFlow(previewMode))
		setActiveRecording(flowRecording)
		await runPreview(previewArgs.val, undefined)
	}

	function collectSubJobIds(flowStatus: Job['flow_status']): string[] {
		if (!flowStatus) return []
		const ids: string[] = []
		for (const mod of flowStatus.modules ?? []) {
			if (mod.job) ids.push(mod.job)
			if (mod.flow_jobs) ids.push(...mod.flow_jobs)
		}
		if (flowStatus.failure_module?.job) ids.push(flowStatus.failure_module.job)
		if (flowStatus.preprocessor_module?.job) ids.push(flowStatus.preprocessor_module.job)
		return ids
	}

	async function recordSubJobs(completedJob: Job) {
		const subJobIds = collectSubJobIds(completedJob.flow_status)
		await Promise.all(
			subJobIds.map(async (subId) => {
				try {
					const subJob = await JobService.getJob({
						workspace: $workspaceStore!,
						id: subId
					})
					flowRecording.addCompletedJob(subId, subJob)
					// Recurse into nested flows (flow-within-flow)
					if (subJob.flow_status) {
						await recordSubJobs(subJob)
					}
				} catch (e) {
					console.warn('[recording] failed to fetch sub-job', subId, e)
				}
			})
		)
	}

	function downloadRecording() {
		if (lastRecording) {
			flowRecording.download(lastRecording)
		}
	}

	let scrollableDiv: HTMLDivElement | undefined = $state(undefined)
	function handleScroll() {
		let newScroll = scrollableDiv?.scrollTop ?? 0
		if (newScroll != 0 && render) {
			scrollTop = newScroll
		}
	}

	function onScrollableDivChange() {
		if (scrollTop != 0 && scrollableDiv) {
			scrollableDiv.scrollTop = scrollTop
		}
	}
	$effect.pre(() => {
		selectedJobStep !== undefined && untrack(() => onSelectedJobStepChange())
	})
	$effect(() => {
		scrollableDiv && render && untrack(() => onScrollableDivChange())
	})

	// During recording, watch sub-job SSE streams to capture incremental logs
	$effect(() => {
		// Only watch sub-jobs for the current job (jobId is set after runPreview starts)
		if (flowRecording.active && job?.flow_status && job?.id === jobId) {
			const modules = job.flow_status.modules ?? []
			untrack(() => {
				for (const mod of modules) {
					if (mod.job) {
						flowRecording.watchSubJob(mod.job, $workspaceStore!)
					}
				}
				if (job?.flow_status?.failure_module?.job) {
					flowRecording.watchSubJob(job.flow_status.failure_module.job, $workspaceStore!)
				}
			})
		}
	})

	export async function cancelTest() {
		isRunning = false
		try {
			jobId &&
				(await JobService.cancelQueuedJob({
					workspace: $workspaceStore ?? '',
					id: jobId,
					requestBody: {}
				}))
		} catch {}
	}

	export function getLocalModuleStates() {
		return localModuleStates
	}

	export function getLocalDurationStatuses() {
		return localDurationStatuses
	}

	export function getSuspendStatus() {
		return suspendStatus
	}

	export function getIsRunning() {
		return isRunning
	}

	export function getIsOwner() {
		return isOwner
	}

	export function getJob() {
		return job
	}

	export function flowHasChanged() {
		return !!lastPreviewFlow && JSON.stringify(flowStore.val) != lastPreviewFlow
	}
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="flex flex-col space-y-2 h-screen bg-surface py-2 w-full" id="flow-preview-content">
	{#if render}
		<div class="flex flex-row w-full items-center gap-x-2 px-4">
			<div class="w-8">
				<Button
					on:click={() => dispatch('close')}
					startIcon={{ icon: X }}
					iconOnly
					unifiedSize="md"
					variant="default"
					btnClasses="hover:bg-surface-hover  bg-surface-secondaryw-8 h-8 rounded-full p-0"
				/>
			</div>

			{#if isRunning}
				<div class="mx-auto flex items-center gap-2">
					<Button
						variant="accent"
						destructive
						on:click={async () => {
							cancelTest()
						}}
						unifiedSize="md"
						btnClasses="w-full max-w-lg"
						loading={true}
						clickableWhileLoading
					>
						Cancel
					</Button>
					{#if flowRecording.active}
						<span class="text-xs text-red-500 font-medium flex items-center gap-1">
							<Circle size={8} fill="currentColor" /> Recording
						</span>
					{/if}
				</div>
			{:else}
				<div class="grow justify-center flex flex-row gap-2 items-center">
					{#if jobId !== undefined && selectedJobStep !== undefined && selectedJobStepIsTopLevel}
						<FlowRestartButton
							{jobId}
							{selectedJobStep}
							{selectedJobStepType}
							{restartBranchNames}
							onRestart={(stepId, branchOrIterationN) => {
								runPreview(previewArgs.val, {
									flow_job_id: jobId,
									step_id: stepId,
									branch_or_iteration_n: branchOrIterationN
								})
							}}
						/>
					{/if}
					{#if !flowStore.val.value?.chat_input_enabled}
						<Button
							variant="accent"
							startIcon={{ icon: isRunning ? RefreshCw : Play }}
							size="sm"
							btnClasses="w-full max-w-lg"
							on:click={() => recordingMode ? recordAndTest() : runPreview(previewArgs.val, undefined)}
							id="flow-editor-test-flow-drawer"
							shortCut={{ Icon: CornerDownLeft }}
						>
							{#if previewMode == 'upTo'}
								Test up to
								<Badge baseClass="ml-1" color="indigo">
									{selectionManager.getSelectedId()}
								</Badge>
							{:else if recordingMode}
								Test flow and record
							{:else}
								Test flow
							{/if}
						</Button>
					{/if}
					{#if lastRecording && recordingMode}
						<Button
							variant="subtle"
							unifiedSize="sm"
							title="Download recording"
							on:click={downloadRecording}
							startIcon={{ icon: Download }}
						>
							Download recording
						</Button>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
	<div
		bind:this={scrollableDiv}
		class="overflow-y-auto grow flex flex-col pt-4 px-4"
		onscroll={(e) => handleScroll()}
	>
		{#if render}
			{#if flowStore.val.value?.chat_input_enabled}
				<div class="flex flex-row justify-center w-full mb-6">
					<FlowChat
						useStreaming={shouldUseStreaming}
						onRunFlow={async (userMessage, conversationId, additionalInputs) => {
							await runPreview(
								{ user_message: userMessage, ...(additionalInputs ?? {}) },
								undefined,
								conversationId
							)
							return jobId ?? ''
						}}
						hideSidebar={true}
						path={$pathStore}
						inputSchema={flowStore.val.schema}
					/>
				</div>
			{:else}
				<div>
					<SchemaFormWithArgPicker
						bind:this={schemaFormWithArgPicker}
						runnableId={$initialPathStore}
						stablePathForCaptures={$initialPathStore || fakeInitialPath}
						runnableType={'FlowPath'}
						previewArgs={previewArgs.val}
						on:openTriggers
						on:select={(e) => {
							selectInput(e.detail.payload, e.detail?.type)
						}}
						{isValid}
						{jsonView}
					>
						<div class="w-full flex flex-row justify-between">
							<InputSelectedBadge
								onReject={() => schemaFormWithArgPicker?.resetSelected()}
								{inputSelected}
							/>
							<div class="flex flex-row gap-2">
								<Toggle
									bind:checked={jsonView}
									size="xs"
									options={{
										right: 'JSON',
										rightTooltip: 'Fill args from JSON'
									}}
									lightMode
									on:change={(e) => {
										jsonEditor?.setCode(JSON.stringify(previewArgs.val ?? {}, null, '\t'))
										refresh()
									}}
								/>
							</div>
						</div>
						{#if jsonView}
							<div class="py-2" style="height: {Math.max(schemaHeight, 100)}px" data-schema-picker>
								<JsonInputs
									bind:this={jsonEditor}
									on:select={(e) => {
										if (e.detail) {
											previewArgs.val = e.detail
										}
									}}
									updateOnBlur={false}
									placeholder={`Write args as JSON.<br/><br/>Example:<br/><br/>{<br/>&nbsp;&nbsp;"foo": "12"<br/>}`}
								/>
							</div>
						{:else}
							{#key renderCount}
								<div bind:clientHeight={schemaHeight} class="min-h-[40vh]">
									<SchemaForm
										noVariablePicker
										compact
										schema={flowStore.val.schema}
										bind:args={previewArgs.val}
										on:change={() => {
											savedArgs = $state.snapshot(previewArgs.val)
										}}
										bind:isValid
										helperScript={flowStore.val.schema?.['x-windmill-dyn-select-code'] &&
										flowStore.val.schema?.['x-windmill-dyn-select-lang']
											? {
													source: 'inline',
													code: flowStore.val.schema['x-windmill-dyn-select-code'] as string,
													lang: flowStore.val.schema['x-windmill-dyn-select-lang'] as ScriptLang
												}
											: undefined}
									/>
								</div>
							{/key}
						{/if}
					</SchemaFormWithArgPicker>
				</div>
			{/if}
		{/if}
		<div class="pt-4 flex flex-col border-t relative">
			{#if flowHasChanged()}
				<div class="pb-2">
					<div
						class="text-xs bg-orange-100 text-orange-700 border border-orange-700/30 p-2 flex items-center gap-2 rounded"
					>
						<AlertTriangle size={12} /> Flow changed since last preview
						<div class="flex"></div>
					</div>
				</div>
			{/if}

			<div class="w-full flex pb-2 items-start justify-between gap-4">
				{#if job}
					<JobDetailHeader {job} compact={true} />
				{:else}
					<div></div> <!-- empty div to keep the same layout -->
				{/if}
				<FlowHistoryJobPicker
					selectInitial={jobId == undefined}
					on:select={(e) => {
						if (!currentJobId) {
							currentJobId = jobId
						}
						const detail = e.detail
						jobId = detail.jobId
						if (detail.initial && stepHistoryLoader?.flowJobInitial === undefined) {
							stepHistoryLoader?.setFlowJobInitial(detail.initial)
						}
					}}
					on:unselect={() => {
						jobId = currentJobId
						currentJobId = undefined
					}}
					path={$initialPathStore == '' ? $pathStore : $initialPathStore}
					bind:loading={loadingHistory}
				/>
			</div>

			<FlowProgressBar {job} bind:this={flowProgressBar} slim textPosition="bottom" showStepId />

			{#if job}
				<div class="w-full my-6">
					<FlowExecutionStatus
						{job}
						workspaceId={$workspaceStore}
						{isOwner}
						innerModules={job?.flow_status?.modules}
						{suspendStatus}
					/>
				</div>
			{/if}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			{#if jobId}
				{#if stepHistoryLoader?.flowJobInitial}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						onclick={() => {
							stepHistoryLoader?.setFlowJobInitial(false)
						}}
						class="cursor-pointer h-full hover:bg-gray-500/20 dark:hover:bg-gray-500/20 dark:bg-gray-500/80 rounded bg-gray-500/40 absolute top-0 left-0 w-full z-50"
					>
						<div class="text-center text-primary text-lg py-2 pt-20"
							><span class="font-bold border p-2 bg-surface-secondary rounded-md"
								>Previous run of this flow from history</span
							></div
						>
					</div>
				{/if}
				<FlowStatusViewer
					bind:job
					bind:localModuleStates
					bind:localDurationStatuses
					{suspendStatus}
					hideDownloadInGraph={customUi?.downloadLogs === false}
					wideResults
					bind:flowState={flowStateStore.val}
					{jobId}
					onDone={async ({ job: completedJob }) => {
						isRunning = false
						$executionCount = $executionCount + 1
						if (flowRecording.active) {
							lastRecording = flowRecording.stop()
							setActiveRecording(undefined)
							// Fetch sub-job data and add to recording (they load after the root completes)
							await recordSubJobs(completedJob)
							// Trigger reactivity update for lastRecording
							lastRecording = lastRecording
						}
						onJobDone?.()
					}}
					bind:selectedJobStep
					bind:rightColumnSelect
					bind:isOwner
					{render}
					{customUi}
					showLogsWithResult
				/>
			{:else if loadingHistory}
				<div class="italic text-primary h-full grow mx-auto flex flex-row items-center gap-2">
					<Loader2 class="animate-spin" /> <span> Loading history... </span>
				</div>
			{/if}
		</div>
	</div>
</div>
