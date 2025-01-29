<script lang="ts">
	import { type Job, JobService, type Flow, type RestartedFrom, type OpenFlow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button, Popup } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SchemaFormWithArgPicker from './SchemaFormWithArgPicker.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import {
		AlertTriangle,
		ArrowRight,
		CornerDownLeft,
		Play,
		RefreshCw,
		X,
		ChevronRight,
		ChevronLeft,
		Library
	} from 'lucide-svelte'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { dfs } from './flows/dfs'
	import { sliceModules } from './flows/flowStateUtils'
	import InputSelectedBadge from './schema/InputSelectedBadge.svelte'
	import { ButtonType } from '$lib/components/common/button/model'
	import { twMerge } from 'tailwind-merge'
	import Toggle from './Toggle.svelte'
	import JsonInputs from './JsonInputs.svelte'

	export let previewMode: 'upTo' | 'whole'
	export let open: boolean
	export let preventEscape = false

	export let jobId: string | undefined = undefined
	export let job: Job | undefined = undefined
	let selectedJobStep: string | undefined = undefined
	let branchOrIterationN: number = 0
	let restartBranchNames: [number, string][] = []

	let selectedJobStepIsTopLevel: boolean | undefined = undefined
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = 'single'

	let isRunning: boolean = false
	let jobProgressReset: () => void
	let jsonView: boolean = false
	let jsonEditor: JsonInputs
	let schemaHeight = 0
	let isValid: boolean = true

	export function test() {
		renderCount++
		runPreview($previewArgs, undefined)
	}

	const {
		selectedId,
		previewArgs,
		flowStateStore,
		flowStore,
		pathStore,
		initialPath,
		customUi,
		executionCount
	} = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	function extractFlow(previewMode: 'upTo' | 'whole'): OpenFlow {
		if (previewMode === 'whole') {
			return $flowStore
		} else {
			const flow: Flow = JSON.parse(JSON.stringify($flowStore))
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
		try {
			lastPreviewFlow = JSON.stringify($flowStore)
			jobProgressReset()
			const newFlow = extractFlow(previewMode)
			jobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom)
			isRunning = true
			if (inputSelected) {
				savedArgs = $previewArgs
				inputSelected = undefined
			}
		} catch (e) {
			sendUserToast('Could not run preview', true, undefined, e.toString())
			isRunning = false
			jobId = undefined
		}
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Enter':
					if (event.ctrlKey || event.metaKey) {
						event.preventDefault()
						runPreview($previewArgs, undefined)
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

	let savedArgs = $previewArgs
	let inputSelected: 'captures' | 'history' | 'saved' | undefined = undefined
	async function selectInput(input, type?: 'captures' | 'history' | 'saved' | undefined) {
		if (!input) {
			$previewArgs = savedArgs
			inputSelected = undefined
			setTimeout(() => {
				preventEscape = false
			}, 100)
		} else {
			$previewArgs = input
			inputSelected = type
			preventEscape = true
			jsonEditor?.setCode(JSON.stringify($previewArgs ?? {}, null, '\t'))
		}
	}

	export function refresh() {
		renderCount++
	}

	$: if (job?.type === 'CompletedJob') {
		isRunning = false
	}

	$: selectedJobStep !== undefined && onSelectedJobStepChange()

	let renderCount: number = 0
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
								runPreview($previewArgs, {
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
						<Popup floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}>
							<svelte:fragment slot="button">
								<Button
									title={`Re-start this flow from step ${selectedJobStep} (included).`}
									variant="border"
									color="blue"
									startIcon={{ icon: RefreshCw }}
									on:click={() => {
										runPreview($previewArgs, {
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
							<label class="block text-primary">
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
											runPreview($previewArgs, {
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
						</Popup>
					{/if}
				{/if}
				<Button
					variant="contained"
					startIcon={{ icon: isRunning ? RefreshCw : Play }}
					color="dark"
					size="sm"
					btnClasses="w-full max-w-lg"
					on:click={() => runPreview($previewArgs, undefined)}
					id="flow-editor-test-flow-drawer"
					shortCut={{ Icon: CornerDownLeft }}
				>
					Test flow
				</Button>
			</div>
		{/if}
	</div>
	<div class="w-full flex flex-col gap-y-1">
		{#if lastPreviewFlow && JSON.stringify($flowStore) != lastPreviewFlow}
			<div class="pt-1">
				<div
					class="bg-orange-200 text-orange-600 border border-orange-600 p-2 flex items-center gap-2 rounded"
				>
					<AlertTriangle size={14} /> Flow changed since last preview
					<div class="flex" />
				</div>
			</div>
		{/if}
		<FlowProgressBar {job} bind:reset={jobProgressReset} />
	</div>

	<div class="overflow-y-auto grow flex flex-col pt-4">
		<div class="border-b">
			<SchemaFormWithArgPicker
				runnableId={initialPath}
				runnableType={'FlowPath'}
				flowPath={$pathStore}
				previewArgs={$previewArgs}
				on:openTriggers
				on:select={(e) => {
					selectInput(e.detail.payload, e.detail?.type)
				}}
				let:toggleRightPanel
				let:selectedTab
				{isValid}
				{jsonView}
			>
				<div class="w-full flex flex-row justify-between">
					<InputSelectedBadge {inputSelected} />
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
								jsonEditor?.setCode(JSON.stringify($previewArgs ?? {}, null, '\t'))
								refresh()
							}}
						/>
						<button
							on:click={() => {
								toggleRightPanel()
							}}
							title={selectedTab ? 'Close' : 'Open'}
							class={twMerge(ButtonType.ColorVariants.light.border, 'rounded-md border')}
						>
							<div class="p-2 center-center flex flex-row gap-2">
								<Library size={14} />
								<p class="text-2xs">Inputs library</p>
								{#if selectedTab}
									<ChevronLeft size={14} />
								{:else}
									<ChevronRight size={14} />
								{/if}
							</div>
						</button>
					</div>
				</div>
				{#if jsonView}
					<div class="py-2" style="height: {Math.max(schemaHeight, 100)}px" data-schema-picker>
						<JsonInputs
							bind:this={jsonEditor}
							on:select={(e) => {
								if (e.detail) {
									$previewArgs = e.detail
								}
							}}
							updateOnBlur={false}
						/>
					</div>
				{:else}
					{#key renderCount}
						<div bind:clientHeight={schemaHeight} class="min-h-[40vh]">
							<SchemaForm
								noVariablePicker
								compact
								schema={$flowStore.schema}
								bind:args={$previewArgs}
								on:change={() => {
									savedArgs = $previewArgs
								}}
								bind:isValid
							/>
						</div>
					{/key}
				{/if}
			</SchemaFormWithArgPicker>
		</div>
		<div class="pt-4 flex flex-col grow">
			{#if jobId}
				<FlowStatusViewer
					hideDownloadInGraph={customUi?.downloadLogs === false}
					wideResults
					{flowStateStore}
					{jobId}
					on:done={() => {
						console.log('done')
						$executionCount = $executionCount + 1
					}}
					on:jobsLoaded={({ detail }) => {
						job = detail
					}}
					bind:selectedJobStep
				/>
			{:else}
				<div class="italic text-tertiary h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
