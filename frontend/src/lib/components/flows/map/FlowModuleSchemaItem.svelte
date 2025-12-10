<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import Popover from '$lib/components/Popover.svelte'
	import { classNames, type StateStore } from '$lib/utils'
	import {
		Bed,
		Database,
		Gauge,
		Move,
		PhoneIncoming,
		Repeat,
		Square,
		SkipForward,
		Pin,
		X,
		Play,
		Loader2,
		TriangleAlert,
		Timer,
		Maximize2
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { FlowEditorContext } from '../types'
	import { twMerge } from 'tailwind-merge'
	import IdEditorInput from '$lib/components/IdEditorInput.svelte'
	import { dfs } from '../dfs'
	import { dfs as dfsPreviousResults } from '../previousResults'
	import { Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { getDependeeAndDependentComponents } from '../flowExplorer'
	import { replaceId } from '../flowStore.svelte'
	import FlowModuleSchemaItemViewer from './FlowModuleSchemaItemViewer.svelte'
	import type { PropPickerContext } from '$lib/components/prop_picker'
	import OutputPicker from '$lib/components/flows/propPicker/OutputPicker.svelte'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import type { FlowState } from '$lib/components/flows/flowState'
	import { Button } from '$lib/components/common'
	import ModuleTest from '$lib/components/ModuleTest.svelte'
	import { getStepHistoryLoaderContext } from '$lib/components/stepHistoryLoader.svelte'
	import type { Job } from '$lib/gen'
	import {
		getNodeColorClasses,
		aiActionToNodeState,
		type FlowNodeState
	} from '$lib/components/graph'
	import type { ModuleActionInfo } from '$lib/components/flows/flowDiff'
	import DiffActionBar from './DiffActionBar.svelte'
	import { getGraphContext } from '$lib/components/graph/graphContext'

	interface Props {
		selected?: boolean
		deletable?: boolean
		moduleAction: ModuleActionInfo | undefined
		retry?: boolean
		cache?: boolean
		earlyStop?: boolean
		skip?: boolean
		suspend?: boolean
		sleep?: boolean
		mock?:
			| {
					enabled?: boolean
					return_value?: unknown
			  }
			| undefined
		bold?: boolean
		id?: string | undefined
		label: string
		path?: string
		modType?: string | undefined
		nodeState?: FlowNodeState
		concurrency?: boolean
		// TODO: Implement for this one. See how concurrency is implemented.
		debouncing?: boolean
		retries?: number | undefined
		warningMessage?: string | undefined
		isTrigger?: boolean
		editMode?: boolean
		alwaysShowOutputPicker?: boolean
		loopStatus?: { type: 'inside' | 'self'; flow: 'forloopflow' | 'whileloopflow' } | undefined
		icon?: import('svelte').Snippet
		onTestUpTo?: ((id: string) => void) | undefined
		inputTransform?: Record<string, any> | undefined
		onUpdateMock?: (mock: { enabled: boolean; return_value?: unknown }) => void
		onEditInput?: (moduleId: string, key: string) => void
		flowJob?: Job | undefined
		isOwner?: boolean
		enableTestRun?: boolean
		maximizeSubflow?: () => void
	}

	let {
		selected = false,
		deletable = false,
		moduleAction = undefined,
		retry = false,
		cache = false,
		earlyStop = false,
		skip = false,
		suspend = false,
		sleep = false,
		mock = { enabled: false },
		bold = false,
		id = undefined,
		label,
		path = '',
		modType = undefined,
		nodeState,
		concurrency = false,
		debouncing = false,
		retries = undefined,
		warningMessage = undefined,
		isTrigger = false,
		editMode = false,
		alwaysShowOutputPicker = false,
		loopStatus = undefined,
		icon,
		onTestUpTo,
		inputTransform,
		onUpdateMock,
		onEditInput,
		flowJob,
		enableTestRun = false,
		maximizeSubflow = undefined
	}: Props = $props()

	// AI action colors take priority over execution state
	let effectiveState = $derived(aiActionToNodeState(moduleAction?.action) ?? nodeState)
	let colorClasses = $derived(getNodeColorClasses(effectiveState, selected))

	const flowEditorContext = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	const flowInputsStore = flowEditorContext?.flowInputsStore
	const flowStore = flowEditorContext?.flowStore

	const flowGraphContext = getGraphContext()
	const diffManager = flowGraphContext?.diffManager

	let pickableIds: Record<string, any> | undefined = $state(undefined)

	const dispatch = createEventDispatcher()

	const propPickerContext = getContext<PropPickerContext>('PropPickerContext')
	const flowPropPickerConfig = propPickerContext?.flowPropPickerConfig
	const pickablePropertiesFiltered = propPickerContext?.pickablePropertiesFiltered

	$effect(() => {
		pickableIds = $pickablePropertiesFiltered?.priorIds
	})

	let editId = $state(false)

	let newId: string = $state(id ?? '')

	let moduleTest: ModuleTest | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let hover = $state(false)
	let connectingData: any | undefined = $state(undefined)
	let outputPicker: OutputPicker | undefined = $state(undefined)
	let testJob: any | undefined = $state(undefined)
	let outputPickerBarOpen = $state(false)

	let flowStateStore = $derived(flowEditorContext?.flowStateStore)

	let stepHistoryLoader = getStepHistoryLoaderContext()

	function updateConnectingData(
		id: string | undefined,
		pickableIds: Record<string, any> | undefined,
		flowPropPickerConfig: any | undefined,
		flowStateStore: StateStore<FlowState> | undefined
	) {
		if (!id) return
		connectingData =
			flowPropPickerConfig && pickableIds && Object.keys(pickableIds).includes(id)
				? pickableIds[id]
				: (flowStateStore?.val?.[id]?.previewResult ?? {})
	}
	$effect(() => {
		updateConnectingData(id, pickableIds, $flowPropPickerConfig, flowStateStore)
	})

	let isConnectingCandidate = $derived(
		!!id && !!$flowPropPickerConfig && !!pickableIds && Object.keys(pickableIds).includes(id)
	)

	const outputPickerVisible = $derived(
		editMode && (isConnectingCandidate || alwaysShowOutputPicker) && !!id
	)

	const icon_render = $derived(icon)

	let testRunDropdownOpen = $state(false)

	let outputPickerInner: OutputPickerInner | undefined = $state(undefined)
	let historyOpen = $derived.by(() => outputPickerInner?.getHistoryOpen?.() ?? false)
</script>

{#if deletable && id && editId}
	{@const flowStore = flowEditorContext?.flowStore ?? undefined}
	{@const getDeps = getDependeeAndDependentComponents(
		id,
		flowStore?.val?.value.modules ?? [],
		flowStore?.val?.value.failure_module
	)}
	<Drawer bind:open={editId}>
		<DrawerContent title="Edit Step Id {id}" on:close={() => (editId = false)}>
			<div>
				<IdEditorInput
					buttonText="Edit Id "
					btnClasses="!ml-1"
					label=""
					initialId={id}
					acceptUnderScores
					reservedIds={dfs(flowStore?.val?.value.modules ?? [], (x) => x.id)}
					bind:value={newId}
					onSave={({ oldId, newId }) => {
						dispatch('changeId', { id: oldId, newId, deps: getDeps?.dependents ?? {} })
						editId = false
					}}
					onClose={() => {
						editId = false
					}}
				/>
				<div class="mt-8">
					<h3>Step Inputs Replacements</h3>
					<div class="text-2xs text-primary pt-0.5">
						Replace all occurrences of `results.<span class="font-bold">{id}</span>` with{' '}
						results.<span class="font-bold">{newId}</span> in the step inputs of all steps that depend
						on it.
					</div>
					<div class="pt-8 flex flex-col gap-y-4">
						{#if Object.keys(getDeps?.dependents ?? {})?.length > 0}
							{#each Object.entries(getDeps?.dependents ?? {}) as dependents}
								<div>
									<h4>{dependents[0]}</h4>
									<div>
										{#each dependents?.[1] as d}
											<div>
												<span class="font-mono text-sm">{d}</span> &rightarrow;
												<span class="font-mono text-sm">{replaceId(d, id, newId)}</span>
											</div>
										{/each}
									</div>
								</div>
							{/each}
						{:else}
							<div class="text-2xs text-primary"> No dependents </div>
						{/if}
					</div>
				</div>
			</div>
		</DrawerContent>
	</Drawer>
{/if}

{#if deletable && id && flowStore && outputPickerVisible}
	{@const flowStoreVal = flowStore.val}
	{@const mod = flowStoreVal?.value ? dfsPreviousResults(id, flowStoreVal, false)[0] : undefined}
	{#if mod && flowStateStore?.val?.[id]}
		<ModuleTest
			bind:this={moduleTest}
			{mod}
			bind:testIsLoading
			bind:testJob
			onJobDone={() => {
				outputPickerInner?.setJobPreview?.()
			}}
		/>
	{/if}
{/if}

<div class="relative">
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={classNames(
			'w-full module flex rounded-md cursor-pointer max-w-full drop-shadow-base',
			colorClasses.bg
		)}
		style="width: 275px; height: 34px;"
		onmouseenter={() => (hover = true)}
		onmouseleave={() => (hover = false)}
		onpointerdown={stopPropagation(preventDefault((e) => dispatch('pointerdown', e)))}
	>
		{#if id}
			<DiffActionBar moduleId={id} {moduleAction} {diffManager} {flowStore} />
		{/if}
		<div
			class={classNames('absolute z-0 rounded-md outline-offset-0', colorClasses.outline)}
			style={`width: 275px; height: 34px;`}
		></div>
		<div
			class="absolute text-sm right-2 flex flex-row gap-1 z-10 transition-all duration-100"
			style={`bottom: ${outputPickerBarOpen ? '-38px' : '-12px'}`}
		>
			{#if retry}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
					>
						{#if retries}<span class="text-red-400 mr-2">{retries}</span>{/if}
						<Repeat size={12} />
					</div>
					{#snippet text()}
						Retries
					{/snippet}
				</Popover>
			{/if}

			{#if concurrency}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
					>
						<Gauge size={12} />
					</div>
					{#snippet text()}
						Concurrency Limits
					{/snippet}
				</Popover>
			{/if}
			{#if debouncing}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
					>
						<Timer size={12} />
					</div>
					{#snippet text()}
						Debouncing
					{/snippet}
				</Popover>
			{/if}
			{#if cache}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
					>
						<Database size={12} />
					</div>
					{#snippet text()}
						Cached
					{/snippet}
				</Popover>
			{/if}
			{#if earlyStop}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
					>
						<Square size={12} />
					</div>
					{#snippet text()}
						{isTrigger ? 'Stop early if there are no new events' : 'Early stop/break'}
					{/snippet}
				</Popover>
			{/if}
			{#if skip}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
					>
						<SkipForward size={12} />
					</div>
					{#snippet text()}
						Skip
					{/snippet}
				</Popover>
			{/if}
			{#if suspend}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
					>
						<PhoneIncoming size={12} />
					</div>
					{#snippet text()}
						Suspend
					{/snippet}
				</Popover>
			{/if}
			{#if sleep}
				<Popover notClickable>
					<div
						transition:fade|local={{ duration: 200 }}
						class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
					>
						<Bed size={12} />
					</div>
					{#snippet text()}
						Sleep
					{/snippet}
				</Popover>
			{/if}
			{#if mock?.enabled}
				<Popover notClickable>
					<button
						transition:fade|local={{ duration: 200 }}
						class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
						onclick={() => {
							outputPicker?.toggleOpen()
						}}
						data-popover
					>
						<Pin size={12} />
					</button>
					{#snippet text()}
						Pinned
					{/snippet}
				</Popover>
			{/if}
		</div>

		<div class="flex flex-col w-full">
			<FlowModuleSchemaItemViewer
				{label}
				{path}
				{id}
				{deletable}
				{bold}
				bind:editId
				{hover}
				{colorClasses}
			>
				{#snippet icon()}
					{@render icon_render?.()}
				{/snippet}
			</FlowModuleSchemaItemViewer>

			{#if outputPickerVisible}
				<OutputPicker
					bind:this={outputPicker}
					{selected}
					{hover}
					{isConnectingCandidate}
					{historyOpen}
					{inputTransform}
					id={id ?? ''}
					bind:bottomBarOpen={outputPickerBarOpen}
					{loopStatus}
					{onEditInput}
				>
					{#snippet children({ allowCopy, isConnecting, selectConnection })}
						<OutputPickerInner
							{allowCopy}
							prefix={'results'}
							connectingData={isConnecting ? connectingData : undefined}
							{mock}
							{testJob}
							moduleId={id}
							onSelect={selectConnection}
							{onUpdateMock}
							{path}
							{loopStatus}
							rightMargin
							historyOffset={{ mainAxis: 12, crossAxis: -9 }}
							clazz="p-1"
							isLoading={testIsLoading ||
								(id ? stepHistoryLoader?.stepStates[id]?.loadingJobs : false)}
							initial={id ? stepHistoryLoader?.stepStates[id]?.initial : undefined}
							bind:this={outputPickerInner}
						/>
					{/snippet}
				</OutputPicker>
			{/if}
		</div>

		{#if deletable}
			{#if maximizeSubflow !== undefined}
				{@render buttonMaximizeSubflow?.()}
			{/if}

			{#if id !== 'preprocessor'}
				<!-- The `style="will-change: transform;"` fixes a bug in Safari where the close and move
			 		 and delete buttons would get clipped (unless an animation is running) -->
				<div
					class={twMerge('absolute -translate-y-[100%] top-2 right-4 h-7 p-1 min-w-7')}
					style="will-change: transform;"
				>
					<button
						class={twMerge(
							'trash center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
							hover || selected ? 'block' : '!hidden',
							'shadow-md rounded-md',
							'group-hover:block'
						)}
						onclick={stopPropagation(preventDefault((event) => dispatch('move')))}
						title="Move"
					>
						<Move size={12} />
					</button>
				</div>
			{/if}

			<div
				class="absolute -translate-y-[100%] top-2 -right-2 h-7 p-1 min-w-7"
				style="will-change: transform;"
			>
				<button
					class={twMerge(
						'trash center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-red-400 hover:text-white p-1',
						selected || hover ? 'block' : '!hidden',
						'group-hover:block',
						'shadow-md rounded-md'
					)}
					title="Delete"
					onclick={stopPropagation(
						preventDefault((event) => dispatch('delete', { id, type: modType }))
					)}
					onpointerdown={stopPropagation(preventDefault(() => {}))}
				>
					<X size={12} />
				</button>
			</div>

			{#if (id && Object.values($flowInputsStore?.[id]?.flowStepWarnings || {}).length > 0) || Boolean(warningMessage)}
				<Popover
					style="will-change: transform;"
					class={twMerge(
						'absolute -translate-y-[100%] top-1 -left-1',
						'flex items-center justify-center rounded-b-none rounded-md p-1 shadow-md  duration-0 ',
						id &&
							Object.values($flowInputsStore?.[id]?.flowStepWarnings || {})?.some(
								(x) => x.type === 'error'
							)
							? 'border-red-600 text-red-600 bg-red-100 hover:bg-red-300'
							: ' text-yellow-600 bg-yellow-100 hover:bg-yellow-300'
					)}
				>
					{#snippet text()}
						<ul class="list-disc px-2">
							{#if id}
								{#each Object.values($flowInputsStore?.[id]?.flowStepWarnings || {}) as m}
									<li>
										{m.message}
									</li>
								{/each}
							{/if}
						</ul>
					{/snippet}

					<TriangleAlert size={12} strokeWidth={2} />
				</Popover>
			{/if}
		{:else if maximizeSubflow !== undefined}
			{@render buttonMaximizeSubflow?.()}
		{/if}
	</div>

	{#if editMode && enableTestRun && flowJob?.type !== 'QueuedJob'}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="absolute top-1/2 -translate-y-1/2 -translate-x-[100%] -left-[0] flex items-center w-fit px-1 h-9 min-w-9"
			onmouseenter={() => (hover = true)}
			onmouseleave={() => (hover = false)}
		>
			{#if (hover || selected || testRunDropdownOpen) && outputPickerVisible}
				<div transition:fade={{ duration: 100 }}>
					{#if !testIsLoading}
						<Button
							size="xs"
							title="Run"
							variant="default"
							btnClasses="px-1 py-1.5 bg-surface"
							on:click={() => {
								outputPicker?.toggleOpen(true)
								moduleTest?.loadArgsAndRunTest()
							}}
							dropdownItems={[
								{
									label: 'Test up to here',
									onClick: () => {
										if (id) {
											onTestUpTo?.(id)
										}
									}
								}
							]}
							dropdownBtnClasses="!w-3 px-0.5"
							bind:dropdownOpen={testRunDropdownOpen}
						>
							{#if testIsLoading}
								<Loader2 size={12} class="animate-spin" />
							{:else}
								<Play size={12} />
							{/if}
						</Button>
					{:else}
						<Button
							size="xs"
							color="red"
							variant="contained"
							btnClasses="!h-[25.5px] !w-[36px] !p-1.5 gap-0.5"
							on:click={async () => {
								moduleTest?.cancelJob()
							}}
						>
							<Loader2 size={10} class="animate-spin mr-0.5" />
							<X size={14} />
						</Button>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>

{#snippet buttonMaximizeSubflow()}
	<div class="absolute -translate-y-[100%] top-2 right-10 h-7 p-1">
		<button
			title="Expand subflow"
			class={twMerge(
				'center-center text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary p-1',
				'shadow-md rounded-md',
				hover || selected ? 'opacity-100' : 'opacity-50'
			)}
			onclick={(e) => {
				e.stopPropagation()
				e.preventDefault()
				maximizeSubflow?.()
			}}
			onpointerdown={(e) => {
				e.stopPropagation()
				e.preventDefault()
			}}
		>
			<Maximize2 size={12} />
		</button>
	</div>
{/snippet}

<style>
	.module:hover .trash {
		display: flex !important;
	}
</style>
