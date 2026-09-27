<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import { AlertTriangle, ArrowRightToLine, Filter, Flag, Trash2, X } from 'lucide-svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import type { ContextMenuItem } from '$lib/components/common/contextmenu/ContextMenu.svelte'
	import StepContextMenu from './StepContextMenu.svelte'
	import { getNodeColorClasses } from '../util'
	import { getFlowRunStatusContext } from '../flowRunStatus.svelte'
	import { SIMPLIFIED, type SimplifiedNodeData } from './simplifiedLayout'
	import { getSimplifiedEditContext, useStepMenu } from './simplifiedContext'
	import SimplifiedSlot from './SimplifiedSlot.svelte'

	let { id, data, selected }: { id: string; data: SimplifiedNodeData; selected?: boolean } =
		$props()

	const flowRunStatus = getFlowRunStatusContext()
	let state = $derived(data.module ? flowRunStatus?.getModuleState(id) : undefined)

	let colors = $derived(
		getNodeColorClasses(
			data.kind === 'input' || data.kind === 'result'
				? '_VirtualItem'
				: state?.skipped
					? '_Skipped'
					: state?.type,
			!!selected
		)
	)

	// Rows are sized in px from the same constants the layout sizes the node with: rem-based
	// classes follow the root font size and would overflow the height the layout reserved.
	const rowStyle = `height: ${SIMPLIFIED.rowHeight}px; line-height: ${SIMPLIFIED.rowHeight}px;`

	const edit = getSimplifiedEditContext()
	const stepMenu = useStepMenu()
	let menuItems = $derived(
		data.kind === 'module' || data.kind === 'preprocessor' ? stepMenu(id, data.module) : []
	)
	let deleteBranch = $derived(
		edit?.editMode && data.deleteBranch
			? () => edit.eventHandlers?.deleteBranch(data.deleteBranch!, data.title)
			: undefined
	)
	let captionMenu: ContextMenuItem[] = $derived(
		deleteBranch
			? [
					{
						id: 'delete-branch',
						label: 'Delete branch',
						icon: Trash2,
						type: 'delete',
						onClick: deleteBranch
					}
				]
			: []
	)
</script>

{#if data.kind === 'slot' && data.insert}
	<SimplifiedSlot insert={data.insert} />
{:else if (data.kind === 'junction' || data.kind === 'empty') && data.insert}
	<!-- In the editor these mark where a step goes, so they become the "+" itself -->
	<div
		class="absolute"
		style="left: {data.width / 2 - 10}px; top: {data.height / 2 - 10}px; width: 20px; height: 20px;"
	>
		<SimplifiedSlot insert={data.insert} />
	</div>
{:else if data.kind === 'caption'}
	<StepContextMenu items={captionMenu}>
		<div
			class="group/caption flex items-center gap-1 text-2xs leading-[14px] text-secondary cursor-default"
			style="width: {data.width}px;"
			title={data.title}
		>
			<span class="truncate">{data.title}</span>
			{#if deleteBranch}
				<button
					type="button"
					title="Delete branch"
					class="nodrag shrink-0 opacity-0 group-hover/caption:opacity-100 text-secondary hover:text-red-500"
					onclick={deleteBranch}
				>
					<X size={11} />
				</button>
			{/if}
		</div>
	</StepContextMenu>
{:else if data.kind === 'junction'}
	<div
		class="rounded-full bg-gray-400 dark:bg-gray-500"
		style="width: {data.width}px; height: {data.height}px;"
	></div>
{:else if data.kind === 'empty'}
	<div
		class="rounded-full bg-component-virtual-node border border-border-normal"
		style="width: {data.width}px; height: {data.height}px;"
		title="Empty branch"
	></div>
{:else}
	<StepContextMenu items={menuItems}>
		<div
			class={twMerge(
				'flex items-center rounded-md cursor-pointer drop-shadow-sm',
				colors.bg,
				colors.outline
			)}
			style="width: {data.width}px; height: {data.height}px; padding: 0 {SIMPLIFIED.paddingX}px; gap: {SIMPLIFIED.badgeGap}px;"
			title={data.title}
		>
			<div
				class="shrink-0 flex items-center justify-center text-secondary"
				style="width: {SIMPLIFIED.badgeSize}px; height: {SIMPLIFIED.badgeSize}px;"
			>
				{#if data.module}
					<FlowModuleIcon module={data.module} size={16} />
				{:else if data.kind === 'input'}
					<ArrowRightToLine size={16} />
				{:else if data.kind === 'result'}
					<Flag size={16} />
				{/if}
			</div>
			<div class="flex flex-col min-w-0 flex-1">
				<div class={twMerge('text-xs font-medium truncate', colors.text)} style={rowStyle}>
					{data.title}
				</div>
				{#if data.stepId || data.typeLabel}
					<div class="text-2xs text-secondary truncate flex items-center gap-1.5" style={rowStyle}>
						{#if data.kind === 'failure'}
							<AlertTriangle size={11} class="shrink-0 text-red-500" />
						{:else if data.kind === 'preprocessor'}
							<Filter size={11} class="shrink-0" />
						{/if}
						{#if data.stepId}
							<span class="font-mono text-primary">{data.stepId}</span>
						{/if}
						{#if data.stepId && data.typeLabel}
							<span class="text-hint">·</span>
						{/if}
						{#if data.typeLabel}
							<span class="truncate">{data.typeLabel}</span>
						{/if}
					</div>
				{/if}
				{#if data.detail}
					<div class="text-2xs font-mono text-hint truncate" style={rowStyle} title={data.detail}>
						{data.detail}
					</div>
				{/if}
			</div>
		</div>
	</StepContextMenu>
{/if}

<Handle type="target" position={Position.Left} isConnectable={false} class="!opacity-0" />
<Handle type="source" position={Position.Right} isConnectable={false} class="!opacity-0" />
