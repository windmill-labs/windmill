<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import Popover from '$lib/components/Popover.svelte'
	import { classNames } from '$lib/utils'
	import {
		AlertTriangle,
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
		Loader2
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext, untrack } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { FlowEditorContext, FlowInput } from '../types'
	import { type Writable } from 'svelte/store'
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

	interface Props {
		selected?: boolean
		deletable?: boolean
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
		bgColor?: string
		bgHoverColor?: string
		concurrency?: boolean
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
	}

	let {
		selected = false,
		deletable = false,
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
		bgColor = '',
		bgHoverColor = '',
		concurrency = false,
		retries = undefined,
		warningMessage = undefined,
		isTrigger = false,
		editMode = false,
		alwaysShowOutputPicker = false,
		loopStatus = undefined,
		icon,
		onTestUpTo,
		inputTransform,
		onUpdateMock
	}: Props = $props()

	let pickableIds: Record<string, any> | undefined = $state(undefined)

	const { flowInputsStore } = getContext<{ flowInputsStore: Writable<FlowInput | undefined> }>(
		'FlowGraphContext'
	)

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')

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
	let lastJob: any | undefined = $state(undefined)
	let outputPicker: OutputPicker | undefined = $state(undefined)
	let historyOpen = $state(false)
	let testJob: any | undefined = $state(undefined)
	let outputPickerBarOpen = $state(false)

	let flowStateStore = $derived(flowEditorContext?.flowStateStore)

	function updateConnectingData(
		id: string | undefined,
		pickableIds: Record<string, any> | undefined,
		flowPropPickerConfig: any | undefined,
		flowStateStore: FlowState | undefined
	) {
		if (!id) return
		connectingData =
			flowPropPickerConfig && pickableIds && Object.keys(pickableIds).includes(id)
				? pickableIds[id]
				: (flowStateStore?.[id]?.previewResult ?? {})
	}
	$effect(() => {
		const args = [id, pickableIds, $flowPropPickerConfig, $flowStateStore] as const
		untrack(() => updateConnectingData(...args))
	})

	function updateLastJob(flowStateStore: any | undefined) {
		if (!flowStateStore || !id || flowStateStore[id]?.previewResult === 'never tested this far') {
			return
		}
		lastJob = {
			id: flowStateStore[id]?.previewJobId ?? '',
			result: flowStateStore[id]?.previewResult,
			type: 'CompletedJob' as const,
			workspace_id: flowStateStore[id]?.previewWorkspaceId ?? '',
			success: flowStateStore[id]?.previewSuccess ?? undefined
		}
	}

	$effect(() => {
		flowStateStore && $flowStateStore && untrack(() => updateLastJob($flowStateStore))
	})

	let nlastJob = $derived.by(() => {
		if (testJob) {
			return { ...testJob, preview: true }
		}
		if (lastJob) {
			return { ...lastJob, preview: false }
		}
		return undefined
	})

	let isConnectingCandidate = $derived(
		!!id && !!$flowPropPickerConfig && !!pickableIds && Object.keys(pickableIds).includes(id)
	)

	const outputPickerVisible = $derived(
		editMode && (isConnectingCandidate || alwaysShowOutputPicker) && !!id
	)

	const icon_render = $derived(icon)
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
					on:save={(e) => {
						dispatch('changeId', { id, newId: e.detail, deps: getDeps?.dependents ?? {} })
						editId = false
					}}
					on:close={() => {
						editId = false
					}}
				/>
				<div class="mt-8">
					<h3>Step Inputs Replacements</h3>
					<div class="text-2xs text-tertiary pt-0.5">
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
							<div class="text-2xs text-tertiary"> No dependents </div>
						{/if}
					</div>
				</div>
			</div>
		</DrawerContent>
	</Drawer>
{/if}

{#if deletable && id && flowEditorContext?.flowStore}
	{@const flowStore = flowEditorContext?.flowStore ? flowEditorContext?.flowStore.val : undefined}
	{@const mod = flowStore?.value ? dfsPreviousResults(id, flowStore, false)[0] : undefined}
	{#if mod && $flowStateStore[id]}
		<ModuleTest
			bind:this={moduleTest}
			{mod}
			bind:testIsLoading
			onJobDone={(job) => {
				testJob = job
				outputPicker?.toggleOpen(true)
			}}
		/>
	{/if}
{/if}

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={classNames('w-full module flex rounded-sm cursor-pointer max-w-full ', 'flex relative')}
	style="width: 275px; height: 34px; background-color: {hover && bgHoverColor
		? bgHoverColor
		: bgColor};"
	onmouseenter={() => (hover = true)}
	onmouseleave={() => (hover = false)}
	onpointerdown={stopPropagation(preventDefault(() => dispatch('pointerdown')))}
>
	<div
		class={classNames(
			'absolute rounded-sm outline-offset-0 outline-slate-500 dark:outline-gray-400',
			selected ? 'outline outline-2' : 'active:outline active:outline-2'
		)}
		style={`width: 275px; height: ${outputPickerVisible ? '51px' : '34px'};`}
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
		<FlowModuleSchemaItemViewer {label} {path} {id} {deletable} {bold} bind:editId {hover}>
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
			>
				{#snippet children({ allowCopy, isConnecting, selectConnection })}
					<OutputPickerInner
						{allowCopy}
						prefix={'results'}
						connectingData={isConnecting ? connectingData : undefined}
						{mock}
						lastJob={nlastJob}
						moduleId={id}
						onSelect={selectConnection}
						{onUpdateMock}
						{path}
						{loopStatus}
						rightMargin
						bind:derivedHistoryOpen={historyOpen}
						historyOffset={{ mainAxis: 12, crossAxis: -9 }}
						clazz="p-1"
					/>
				{/snippet}
			</OutputPicker>
		{/if}
	</div>

	{#if deletable}
		<div class="absolute top-1/2 -translate-y-1/2 -left-[54px] center-center w-12 h-9">
			{#if hover || selected}
				<div transition:fade={{ duration: 100 }}>
					<Button
						size="sm"
						color="dark"
						title="Run"
						btnClasses="p-1.5"
						on:click={() => {
							moduleTest?.loadArgsAndRunTest()
						}}
						dropdownItems={[
							{
								label: 'test up to here',
								onClick: () => {
									if (id) {
										onTestUpTo?.(id)
									}
								}
							}
						]}
						dropdownBtnClasses="!w-4 px-1"
					>
						{#if testIsLoading}
							<Loader2 size={12} class="animate-spin" />
						{:else}
							<Play size={12} />
						{/if}
					</Button>
				</div>
			{/if}
		</div>
		<button
			class="absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-0 hover:bg-red-400 hover:text-white
 {hover || selected ? '' : '!hidden'}"
			title="Delete"
			onclick={stopPropagation(
				preventDefault((event) => dispatch('delete', { id, type: modType }))
			)}
		>
			<X class="mx-[3px]" size={12} strokeWidth={2} />
		</button>

		{#if id !== 'preprocessor'}
			<button
				class="absolute -top-[10px] right-[60px] rounded-full h-[20px] w-[20px] trash center-center text-secondary
outline-[1px] outline dark:outline-gray-500 outline-gray-300 bg-surface duration-0 hover:bg-blue-400 hover:text-white
 {hover ? '' : '!hidden'}"
				onclick={stopPropagation(preventDefault((event) => dispatch('move')))}
				title="Move"
			>
				<Move class="mx-[3px]" size={12} strokeWidth={2} />
			</button>
		{/if}

		{#if (id && Object.values($flowInputsStore?.[id]?.flowStepWarnings || {}).length > 0) || Boolean(warningMessage)}
			<div class="absolute -top-[10px] -left-[10px]">
				<Popover>
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
					<div
						class={twMerge(
							'flex items-center justify-center h-full w-full rounded-md p-0.5 border  duration-0 ',
							id &&
								Object.values($flowInputsStore?.[id]?.flowStepWarnings || {})?.some(
									(x) => x.type === 'error'
								)
								? 'border-red-600 text-red-600 bg-red-100 hover:bg-red-300'
								: 'border-yellow-600 text-yellow-600 bg-yellow-100 hover:bg-yellow-300'
						)}
					>
						<AlertTriangle size={14} strokeWidth={2} />
					</div>
				</Popover>
			</div>
		{/if}
	{/if}
</div>

<style>
	.module:hover .trash {
		display: flex !important;
	}
</style>
