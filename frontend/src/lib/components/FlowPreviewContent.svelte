<script lang="ts">
	import { type Job, JobService, type Flow, type RestartedFrom, type OpenFlow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button } from './common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
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
	import { NEVER_TESTED_THIS_FAR } from './flows/models'
	import { writable, type Writable } from 'svelte/store'
	import type { DurationStatus, GraphModuleState } from './graph'

	export let previewMode: 'upTo' | 'whole'
	export let open: boolean
	export let preventEscape = false

	export let jobId: string | undefined = undefined
	export let job: Job | undefined = undefined
	export let initial: boolean = false

	export let selectedJobStep: string | undefined = undefined
	export let selectedJobStepIsTopLevel: boolean | undefined = undefined
	export let selectedJobStepType: 'single' | 'forloop' | 'branchall' = 'single'
	export let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		'timeline'

	export let branchOrIterationN: number = 0
	export let scrollTop: number = 0

	export let localModuleStates: Writable<Record<string, GraphModuleState>> = writable({})
	export let localDurationStatuses: Writable<Record<string, DurationStatus>> = writable({})
	export let onJobsLoaded: (() => void) | undefined = undefined

	let restartBranchNames: [number, string][] = []

	let isRunning: boolean = false
	let jobProgressReset: () => void
	let jsonView: boolean = false
	let jsonEditor: JsonInputs
	let schemaHeight = 0
	let isValid: boolean = true

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
	} = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	function setAllModulesInitial(isInitial: boolean) {
		// Set all existing modules in flowStateStore to initial value
		if (isInitial) {
			for (const moduleId in $flowStateStore) {
				if ($flowStateStore[moduleId] && $flowStateStore[moduleId].initial === undefined) {
					$flowStateStore[moduleId] = {
						...$flowStateStore[moduleId],
						initial: true
					}
				}
			}
		} else {
			for (const moduleId in $flowStateStore) {
				if ($flowStateStore[moduleId] && $flowStateStore[moduleId].initial) {
					$flowStateStore[moduleId] = {
						...$flowStateStore[moduleId],
						initial: false
					}
				}
			}
		}
		$flowStateStore = $flowStateStore
	}

	let renderCount: number = 0
	let schemaFormWithArgPicker: SchemaFormWithArgPicker | undefined = undefined
	let currentJobId: string | undefined = undefined

	function extractFlow(previewMode: 'upTo' | 'whole'): OpenFlow {
		if (previewMode === 'whole') {
			return flowStore.val
		} else {
			const flow: Flow = JSON.parse(JSON.stringify(flowStore.val))
			const idOrders = dfs(flow.value.modules, (x) => x.id)
			let upToIndex = idOrders.indexOf($selectedId)

			if (upToIndex != -1) {
				flow.value.modules = sliceModules(flow.value.modules, upToIndex, idOrders)
			}
			return flow
		}
	}

	let lastPreviewFlow: undefined | string = undefined
	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined
	) {
		if (initial) {
			initial = false
		}
		try {
			lastPreviewFlow = JSON.stringify(flowStore.val)
			jobProgressReset()
			const newFlow = extractFlow(previewMode)
			jobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom)
			isRunning = true
			if (inputSelected) {
				savedArgs = previewArgs.val
				inputSelected = undefined
			}
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

	let savedArgs = previewArgs.val
	let inputSelected: 'captures' | 'history' | 'saved' | undefined = undefined
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

	$: if (job?.type === 'CompletedJob') {
		isRunning = false
	}

	$: selectedJobStep !== undefined && onSelectedJobStepChange()

	// When initial becomes true, mark all modules as initial
	$: setAllModulesInitial(initial)

	async function loadIndividualStepsStates() {
		// console.log('loadIndividualStepsStates')
		dfs(flowStore.val.value.modules, async (module) => {
			// console.log('module', $flowStateStore[module.id], module.id)
			const prev = $flowStateStore[module.id]?.previewResult
			if (prev && prev != NEVER_TESTED_THIS_FAR) {
				return
			}
			const previousJobId = await JobService.listJobs({
				workspace: $workspaceStore!,
				scriptPathExact:
					`path` in module.value
						? module.value.path
						: ($initialPathStore == '' ? $pathStore : $initialPathStore) + '/' + module.id,
				jobKinds: ['preview', 'script', 'flowpreview', 'flow', 'flowscript'].join(','),
				page: 1,
				perPage: 1
			})
			// console.log('previousJobId', previousJobId, module.id)

			if (previousJobId.length > 0) {
				const getJobResult = await JobService.getCompletedJobResultMaybe({
					workspace: $workspaceStore!,
					id: previousJobId[0].id
				})
				if ('result' in getJobResult) {
					$flowStateStore[module.id] = {
						...($flowStateStore[module.id] ?? {}),
						previewResult: getJobResult.result,
						previewJobId: previousJobId[0].id,
						previewWorkspaceId: previousJobId[0].workspace_id,
						previewSuccess: getJobResult.success,
						initial: true
					}
				}
			}
		})
	}

	let scrollableDiv: HTMLDivElement | undefined = undefined
	function handleScroll() {
		scrollTop = scrollableDiv?.scrollTop ?? 0
	}

	$: scrollableDiv && onScrollableDivChange()

	function onScrollableDivChange() {
		if (scrollTop != 0 && scrollableDiv) {
			scrollableDiv.scrollTop = scrollTop
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<div class="flex flex-col space-y-2 h-screen bg-surface px-4 py-2 w-full" id="flow-preview-content">
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
						isRunning = false
						try {
							jobId &&
								(await JobService.cancelQueuedJob({
									workspace: $workspaceStore ?? '',
									id: jobId,
									requestBody: {}
								}))
						} catch {}
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
				{#if jobId !== undefined && selectedJobStep !== undefined && selectedJobStepIsTopLevel}
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
							<svelte:fragment slot="button">
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
							</svelte:fragment>
							<svelte:fragment slot="content">
								<label class="block text-primary p-4">
									<div class="pb-1 text-sm text-secondary"
										>{selectedJobStepType == 'forloop' ? 'From iteration #:' : 'From branch:'}</div
									>
									<div class="flex w-full">
										{#if selectedJobStepType === 'forloop'}
											<input
												type="number"
												min="0"
												bind:value={branchOrIterationN}
												class="!w-32 grow"
												on:click|stopPropagation={() => {}}
											/>
										{:else}
											<select
												bind:value={branchOrIterationN}
												class="!w-32 grow"
												on:click|stopPropagation={() => {}}
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
							</svelte:fragment>
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
		{#if lastPreviewFlow && JSON.stringify(flowStore.val) != lastPreviewFlow}
			<div class="pt-1">
				<div
					class="bg-orange-200 text-orange-600 border border-orange-600 p-2 flex items-center gap-2 rounded"
				>
					<AlertTriangle size={14} /> Flow changed since last preview
					<div class="flex"></div>
				</div>
			</div>
		{/if}
		<FlowProgressBar {job} bind:reset={jobProgressReset} />
	</div>

	<div
		bind:this={scrollableDiv}
		class="overflow-y-auto grow flex flex-col pt-4"
		on:scroll={(e) => handleScroll()}
	>
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
		<div class="pt-4 flex flex-col grow relative">
			<div
				class="absolute top-[22px] right-2 border p-1.5 hover:bg-surface-hover rounded-md center-center"
			>
				<FlowHistoryJobPicker
					selectInitial={jobId == undefined}
					on:nohistory={() => {
						loadIndividualStepsStates()
					}}
					on:select={(e) => {
						if (!currentJobId) {
							currentJobId = jobId
						}
						const detail = e.detail
						initial = detail.initial
						jobId = detail.jobId
					}}
					on:unselect={() => {
						jobId = currentJobId
						currentJobId = undefined
					}}
					path={$initialPathStore == '' ? $pathStore : $initialPathStore}
				/>
			</div>
			{#if jobId}
				{#if initial}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						on:click={() => {
							initial = false
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
					hideDownloadInGraph={customUi?.downloadLogs === false}
					wideResults
					{flowStateStore}
					{jobId}
					on:done={() => {
						$executionCount = $executionCount + 1
					}}
					on:jobsLoaded={() => {
						if (initial) {
							loadIndividualStepsStates()
							onJobsLoaded?.()
						}
					}}
					bind:selectedJobStep
					bind:rightColumnSelect
				/>
			{:else}
				<div class="italic text-tertiary h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
