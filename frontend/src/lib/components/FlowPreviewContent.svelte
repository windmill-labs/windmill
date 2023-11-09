<script lang="ts">
	import {
		Job,
		JobService,
		type Flow,
		type FlowModule,
		type RestartedFrom,
		type OpenFlow
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button, Drawer, Kbd, Popup } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import CapturePayload from './flows/content/CapturePayload.svelte'
	import { ArrowRight, Play, RefreshCw, X } from 'lucide-svelte'
	import { emptyString, getModifierKey } from '$lib/utils'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import SavedInputs from './SavedInputs.svelte'
	import { dfs } from './flows/dfs'

	let capturePayload: CapturePayload
	export let previewMode: 'upTo' | 'whole'
	export let open: boolean

	export let jobId: string | undefined = undefined
	export let job: Job | undefined = undefined
	let selectedJobStep: string | undefined = undefined
	let branchOrIterationN: number = 0
	let restartBranchNames: [number, string][] = []

	let selectedJobStepIsTopLevel: boolean | undefined = undefined
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = 'single'

	let isRunning: boolean = false
	let jobProgressReset: () => void

	export function test() {
		runPreview($previewArgs, undefined)
	}

	const { selectedId, previewArgs, flowStateStore, flowStore, pathStore, initialPath } =
		getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	function sliceModules(modules: FlowModule[], upTo: number, idOrders: string[]): FlowModule[] {
		return modules
			.filter((x) => idOrders.indexOf(x.id) <= upTo)
			.map((m) => {
				if (idOrders.indexOf(m.id) == upTo) {
					return m
				}
				if (m.value.type === 'forloopflow') {
					m.value.modules = sliceModules(m.value.modules, upTo, idOrders)
				} else if (m.value.type === 'branchone') {
					m.value.branches = m.value.branches.map((b) => {
						b.modules = sliceModules(b.modules, upTo, idOrders)
						return b
					})
					m.value.default = sliceModules(m.value.default, upTo, idOrders)
				} else if (m.value.type === 'branchall') {
					m.value.branches = m.value.branches.map((b) => {
						b.modules = sliceModules(b.modules, upTo, idOrders)
						return b
					})
				}
				return m
			})
	}

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

	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined
	) {
		jobProgressReset()
		const newFlow = extractFlow(previewMode)
		jobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom)
		isRunning = true
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

	$: if (job?.type === 'CompletedJob') {
		isRunning = false
	}

	$: selectedJobStep !== undefined && onSelectedJobStepChange()

	let inputLibraryDrawer: Drawer
</script>

<CapturePayload bind:this={capturePayload} />

<svelte:window on:keydown={onKeyDown} />

<Drawer bind:this={inputLibraryDrawer}>
	<DrawerContent title="Input library {initialPath}" on:close={inputLibraryDrawer?.toggleDrawer}>
		<SavedInputs
			flowPath={initialPath}
			isValid={true}
			args={$previewArgs}
			on:selected_args={(e) => {
				$previewArgs = JSON.parse(JSON.stringify(e.detail))
				inputLibraryDrawer?.closeDrawer()
			}}
		/>
	</DrawerContent>
</Drawer>

<div
	class="flex divide-y flex-col space-y-2 h-screen bg-surface px-6 py-2 w-full"
	id="flow-preview-content"
>
	<div class="flex flex-row justify-between w-full items-center gap-x-2">
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
			>
				Cancel
			</Button>
		{:else}
			<div class="flex flex-row gap-4">
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
				>
					Test flow &nbsp;<Kbd small isModifier>{getModifierKey()}</Kbd>
					<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
				</Button>
			</div>
		{/if}
		<div class="flex gap-2">
			{#if initialPath != ''}
				<Button
					btnClasses="h-full truncate"
					size="sm"
					variant="border"
					on:click={() => {
						inputLibraryDrawer?.openDrawer()
					}}>Past Runs/Input library</Button
				>
			{/if}

			<Button
				btnClasses="h-full truncate"
				size="sm"
				variant="border"
				on:click={() => {
					capturePayload.openDrawer()
				}}>Fill args from a request</Button
			>
		</div>
	</div>
	<FlowProgressBar {job} bind:reset={jobProgressReset} />

	<div class="overflow-y-auto grow divide-y divide-gray-600 pr-4">
		<div class="max-h-1/2 overflow-auto border-b border-gray-700">
			<SchemaForm
				noVariablePicker
				compact
				class="py-4 max-w-3xl"
				schema={$flowStore.schema}
				bind:args={$previewArgs}
			/>
		</div>
		<div class="pt-4 grow">
			{#if jobId}
				<FlowStatusViewer
					{flowStateStore}
					{jobId}
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
