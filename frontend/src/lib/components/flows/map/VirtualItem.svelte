<script lang="ts">
	import { Badge } from '$lib/components/common'

	import VirtualItemWrapper from './VirtualItemWrapper.svelte'
	import OutputPicker from '$lib/components/flows/propPicker/OutputPicker.svelte'
	import OutputPickerInner from '$lib/components/flows/propPicker/OutputPickerInner.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { fade } from 'svelte/transition'
	import { Database, Square } from 'lucide-svelte'
	import ModuleAcceptReject, {
		getAiModuleAction
	} from '$lib/components/copilot/chat/flow/ModuleAcceptReject.svelte'
	import { aiModuleActionToBgColor } from '$lib/components/copilot/chat/flow/utils'
	import FlowGraphPreviewButton from './FlowGraphPreviewButton.svelte'
	import type { Job } from '$lib/gen'

	interface Props {
		label?: string | undefined
		bgColor?: string
		bgHoverColor?: string
		selected: boolean
		selectable: boolean
		id?: string | undefined
		center?: boolean
		borderColor?: string | undefined
		hideId?: boolean
		preLabel?: string | undefined
		inputJson?: Object | undefined
		prefix?: string
		cache?: boolean
		earlyStop?: boolean
		editMode?: boolean
		icon?: import('svelte').Snippet
		onUpdateMock?: (mock: { enabled: boolean; return_value?: unknown }) => void
		onEditInput?: (moduleId: string, key: string) => void
		onTestFlow?: () => void
		isRunning?: boolean
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
		individualStepTests?: boolean
		nodeKind?: 'input' | 'result'
		job?: Job
		type?: string
		showJobStatus?: boolean
	}

	let {
		label = undefined,
		bgColor = '',
		bgHoverColor = '',
		selected,
		selectable,
		id = undefined,
		center = true,
		borderColor = undefined,
		hideId = false,
		preLabel = undefined,
		inputJson = undefined,
		prefix = '',
		nodeKind,
		cache = false,
		earlyStop = false,
		editMode = false,
		icon,
		onUpdateMock,
		onEditInput,
		onTestFlow,
		isRunning,
		onCancelTestFlow,
		onOpenPreview,
		onHideJobStatus,
		individualStepTests = false,
		job,
		showJobStatus = false
	}: Props = $props()

	const outputPickerVisible = $derived(
		(nodeKind || (inputJson && Object.keys(inputJson).length > 0)) && editMode
	)

	let action = $derived(label === 'Input' ? getAiModuleAction(label) : undefined)
	let hoverButton = $state(false)

	const outputType = $derived(
		showJobStatus
			? job?.type === 'QueuedJob'
				? 'InProgress'
				: job?.type === 'CompletedJob'
					? job.success
						? 'Success'
						: 'Failure'
					: undefined
			: undefined
	)
</script>

<VirtualItemWrapper
	{label}
	{bgColor}
	{bgHoverColor}
	{selected}
	{selectable}
	{id}
	outputPickerVisible={outputPickerVisible ?? false}
	className={editMode ? aiModuleActionToBgColor(action) : ''}
	on:select
>
	{#snippet children({ hover })}
		{#if editMode}
			<ModuleAcceptReject id="Input" {action} />
		{/if}
		<div class="flex flex-col w-full">
			<div
				style={borderColor ? `border-color: ${borderColor};` : 'border: 0'}
				class="flex flex-row gap-1 justify-between {center
					? 'items-center'
					: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400 dark:border-gray-600"
			>
				{#if icon}
					{@render icon?.()}
					<span class="mr-2"></span>
				{/if}
				<div class="flex flex-col flex-grow shrink-0 max-w-full min-w-0">
					{#if label}
						<div class="truncate text-center">{label}</div>
					{/if}
					{#if preLabel}
						<div class="truncate text-2xs text-center"><pre>{preLabel}</pre></div>
					{/if}
				</div>
				{#if id && !hideId && !id?.startsWith('subflow:')}
					<div class="flex items-center shrink min-w-0">
						<Badge color="indigo" wrapperClass="w-full" baseClass="max-w-full" title={id}>
							<span class="max-w-full text-2xs truncate">{id}</span>
						</Badge>
					</div>
				{/if}
			</div>
			{#if outputPickerVisible}
				<OutputPicker
					{selected}
					{hover}
					id={id ?? ''}
					isConnectingCandidate={true}
					variant="virtual"
					type={outputType}
				>
					{#snippet children({ allowCopy, isConnecting, selectConnection })}
						<OutputPickerInner
							{allowCopy}
							{prefix}
							connectingData={isConnecting ? inputJson : undefined}
							onSelect={selectConnection}
							moduleId={''}
							{onUpdateMock}
							hideHeaderBar={nodeKind !== 'result'}
							simpleViewer={inputJson}
							rightMargin
							historyOffset={{ mainAxis: 12, crossAxis: -9 }}
							clazz="p-1"
							{onEditInput}
							selectionId={id ?? label ?? ''}
							testJob={job}
							disableMock
							disableHistory
							customEmptyJobMessage={nodeKind === 'result'
								? 'Test the flow to see results'
								: undefined}
						/>
					{/snippet}
				</OutputPicker>
			{/if}
		</div>

		<div class="absolute text-sm right-12 -bottom-3 flex flex-row gap-1 z-10">
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
						Early stop if condition met
					{/snippet}
				</Popover>
			{/if}
		</div>
	{/snippet}
	{#snippet previewButton()}
		{#if nodeKind === 'input'}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="absolute top-1/2 -translate-y-[35px] -translate-x-[100%] -left-[0] flex py-4 justify-end w-fit px-2 min-w-32"
				onmouseenter={() => {
					hoverButton = true
				}}
				onmouseleave={() => {
					hoverButton = false
				}}
			>
				{#if outputPickerVisible}
					<div transition:fade={{ duration: 100 }}>
						<FlowGraphPreviewButton
							{isRunning}
							hover={hoverButton}
							{selected}
							{onTestFlow}
							{onCancelTestFlow}
							{onOpenPreview}
							{onHideJobStatus}
							{individualStepTests}
						/>
					</div>
				{/if}
			</div>
		{/if}
	{/snippet}
</VirtualItemWrapper>
