<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { type Job, JobService, type RestartedFrom, type OpenFlow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button } from './common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SchemaFormWithArgPicker from './SchemaFormWithArgPicker.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import { AlertTriangle, ArrowRight, CornerDownLeft, Play, RefreshCw, X } from 'lucide-svelte'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { dfs } from './flows/dfs'
	import { sliceModules } from './flows/flowStateUtils.svelte'
	import InputSelectedBadge from './schema/InputSelectedBadge.svelte'
	import Toggle from './Toggle.svelte'
	import JsonInputs from './JsonInputs.svelte'
	import FlowHistoryJobPicker from './FlowHistoryJobPicker.svelte'
	import { writable, type Writable } from 'svelte/store'
	import type { DurationStatus, GraphModuleState } from './graph'
	import { getStepHistoryLoaderContext } from './stepHistoryLoader.svelte'
	import { aiChatManager } from './copilot/chat/AIChatManager.svelte'
	import { stateSnapshot } from '$lib/svelte5Utils.svelte'

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
		localModuleStates?: Writable<Record<string, GraphModuleState>>
		localDurationStatuses?: Writable<Record<string, DurationStatus>>
		onRunPreview?: () => void
		render?: boolean
		onJobDone?: () => void
		upToId?: string | undefined
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
		localModuleStates = $bindable(writable({})),
		localDurationStatuses = $bindable(writable({})),
		onRunPreview,
		render = false,
		onJobDone,
		upToId = undefined
	}: Props = $props()

	let restartBranchNames: [number, string][] = []
	let isRunning: boolean = $state(false)
	let jsonView: boolean = $state(false)
	let jsonEditor: JsonInputs | undefined = $state(undefined)
	let schemaHeight = $state(0)
	let isValid: boolean = $state(true)
	let suspendStatus: Writable<Record<string, { job: Job; nb: number }>> = $state(writable({}))
	let isOwner: boolean = $state(false)

	export function test() {
		renderCount++
		runPreview(previewArgs.val, undefined)
	}

	const {
		selectedId,
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

	function extractFlow(previewMode: 'upTo' | 'whole'): OpenFlow {
		const previewFlow = aiChatManager.flowAiChatHelpers?.getPreviewFlow()
		if (previewMode === 'whole') {
			return previewFlow ?? flowStore.val
		} else {
			const flow = previewFlow ?? stateSnapshot(flowStore).val
			const idOrders = dfs(flow.value.modules, (x) => x.id)
			let upToIndex = idOrders.indexOf(upToId ?? $selectedId)

			if (upToIndex != -1) {
				flow.value.modules = sliceModules(flow.value.modules, upToIndex, idOrders)
			}
			return flow
		}
	}

	let lastPreviewFlow: undefined | string = $state(undefined)
	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined
	) {
		if (stepHistoryLoader?.flowJobInitial !== false) {
			stepHistoryLoader?.setFlowJobInitial(false)
		}
		try {
			lastPreviewFlow = JSON.stringify(flowStore.val)
			flowProgressBar?.reset()
			const newFlow = extractFlow(previewMode)
			jobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom)
			isRunning = true
			if (inputSelected) {
				savedArgs = previewArgs.val
				inputSelected = undefined
			}
			onRunPreview?.()
		} catch (e) {
			sendUserToast('Could not run preview', true, undefined, e.toString())
			isRunning = false
			jobId = undefined
		}
		schemaFormWithArgPicker?.refreshHistory()
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
						event.stopPropagation
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

<div class="flex flex-col space-y-2 h-screen bg-surface px-4 py-2 w-full" id="flow-preview-content">
	{#if render}
		<div class="flex flex-row w-full items-center gap-x-2">
			<div class="w-8">
				<Button
					on:click={() => dispatch('close')}
					startIcon={{ icon: X }}
					iconOnly
					size="sm"
					color="light"
					btnClasses="hover:bg-surface-hover  bg-surface-secondaryw-8 h-8 rounded-full p-0"
				/>
			</div>

			{#if isRunning}
				<div class="mx-auto">
					<Button
						color="red"
						on:click={async () => {
							cancelTest()
						}}
						size="sm"
						btnClasses="w-full max-w-lg"
						loading={true}
						clickableWhileLoading
					>
						Cancel
					</Button>
				</div>
			{:else}
				<div class="grow justify-center flex flex-row gap-4">
					{#if jobId !== undefined && selectedJobStep !== undefined && selectedJobStepIsTopLevel && aiChatManager.flowAiChatHelpers?.getModuleAction(selectedJobStep) !== 'removed'}
						{#if selectedJobStepType == 'single'}
							<Button
								size="xs"
								color="light"
								variant="border"
								title={`Re-start this flow from step ${selectedJobStep} (included).`}
								on:click={() => {
									runPreview(previewArgs.val, {
										flow_job_id: jobId,
										step_id: selectedJobStep,
										branch_or_iteration_n: 0
									})
								}}
								startIcon={{ icon: Play }}
							>
								Re-start from
								<Badge baseClass="ml-1" color="indigo">
									{selectedJobStep}
								</Badge>
							</Button>
						{:else}
							<Popover
								floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
								contentClasses="p-4"
							>
								{#snippet button()}
									<Button
										title={`Re-start this flow from step ${selectedJobStep} (included).`}
										variant="border"
										color="blue"
										startIcon={{ icon: RefreshCw }}
										on:click={() => {
											runPreview(previewArgs.val, {
												flow_job_id: jobId,
												step_id: selectedJobStep,
												branch_or_iteration_n: 0
											})
										}}
										nonCaptureEvent={true}
									>
										Re-start from
										<Badge baseClass="ml-1" color="indigo">
											{selectedJobStep}
										</Badge>
									</Button>
								{/snippet}
								{#snippet content()}
									<label class="block text-primary p-4">
										<div class="pb-1 text-sm text-secondary"
											>{selectedJobStepType == 'forloop'
												? 'From iteration #:'
												: 'From branch:'}</div
										>
										<div class="flex w-full">
											{#if selectedJobStepType === 'forloop'}
												<input
													type="number"
													min="0"
													bind:value={branchOrIterationN}
													class="!w-32 grow"
													onclick={stopPropagation(() => {})}
												/>
											{:else}
												<select
													bind:value={branchOrIterationN}
													class="!w-32 grow"
													onclick={stopPropagation(() => {})}
												>
													{#each restartBranchNames as [branchIdx, branchName]}
														<option value={branchIdx}>{branchName}</option>
													{/each}
												</select>
											{/if}
											<Button
												size="xs"
												color="blue"
												buttonType="button"
												btnClasses="!p-1 !w-[34px] !ml-1"
												aria-label="Restart flow"
												on:click|once={() => {
													runPreview(previewArgs.val, {
														flow_job_id: jobId,
														step_id: selectedJobStep,
														branch_or_iteration_n: branchOrIterationN
													})
												}}
											>
												<ArrowRight size={18} />
											</Button>
										</div>
									</label>
								{/snippet}
							</Popover>
						{/if}
					{/if}
					<Button
						variant="contained"
						startIcon={{ icon: isRunning ? RefreshCw : Play }}
						color="dark"
						size="sm"
						btnClasses="w-full max-w-lg"
						on:click={() => runPreview(previewArgs.val, undefined)}
						id="flow-editor-test-flow-drawer"
						shortCut={{ Icon: CornerDownLeft }}
					>
						{#if previewMode == 'upTo'}
							Test up to
							<Badge baseClass="ml-1" color="indigo">
								{$selectedId}
							</Badge>
						{:else}
							Test flow
						{/if}
					</Button>
				</div>
			{/if}
		</div>

		<div class="w-full flex flex-col gap-y-1">
			{#if flowHasChanged()}
				<div class="pt-1">
					<div
						class="bg-orange-200 text-orange-600 border border-orange-600 p-2 flex items-center gap-2 rounded"
					>
						<AlertTriangle size={14} /> Flow changed since last preview
						<div class="flex"></div>
					</div>
				</div>
			{/if}
			<FlowProgressBar {job} bind:this={flowProgressBar} />
		</div>
	{/if}
	<div
		bind:this={scrollableDiv}
		class="overflow-y-auto grow flex flex-col pt-4"
		onscroll={(e) => handleScroll()}
	>
		{#if render}
			<div class="border-b">
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
								label="JSON View"
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
										savedArgs = previewArgs.val
									}}
									bind:isValid
								/>
							</div>
						{/key}
					{/if}
				</SchemaFormWithArgPicker>
			</div>
		{/if}
		<div class="pt-4 flex flex-col grow relative">
			<div
				class="absolute top-[22px] right-2 border p-1.5 hover:bg-surface-hover rounded-md center-center"
			>
				{#if render}
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
					/>
				{/if}
			</div>
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
					bind:suspendStatus
					hideDownloadInGraph={customUi?.downloadLogs === false}
					wideResults
					{flowStateStore}
					{jobId}
					on:done={() => {
						isRunning = false
						$executionCount = $executionCount + 1
						onJobDone?.()
					}}
					bind:selectedJobStep
					bind:rightColumnSelect
					bind:isOwner
					{render}
				/>
			{:else}
				<div class="italic text-tertiary h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
