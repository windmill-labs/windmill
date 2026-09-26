<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import { AlertTriangle, ArrowRightToLine, Filter, Flag } from 'lucide-svelte'
	import FlowModuleIcon from '$lib/components/flows/FlowModuleIcon.svelte'
	import { getNodeColorClasses } from '../util'
	import { getFlowRunStatusContext } from '../flowRunStatus.svelte'
	import { SIMPLIFIED, type SimplifiedNodeData } from './simplifiedLayout'

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
</script>

{#if data.kind === 'caption'}
	<div
		class="text-2xs leading-[14px] text-secondary truncate cursor-default"
		style="width: {data.width}px;"
		title={data.title}
	>
		{data.title}
	</div>
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
{:else}<div
		class={twMerge(
			'flex items-center rounded-md cursor-pointer drop-shadow-sm',
			colors.bg,
			colors.outline
		)}
		style="width: {data.width}px; height: {data.height}px; padding: 0 {SIMPLIFIED.paddingX}px; gap: {SIMPLIFIED.badgeGap}px;"
		title={data.title}
	>
		<div
			class="shrink-0 flex items-center justify-center rounded-md bg-surface-tertiary border border-border-light text-secondary"
			style="width: {SIMPLIFIED.badgeSize}px; height: {SIMPLIFIED.badgeSize}px;"
		>
			{#if data.module}
				<FlowModuleIcon module={data.module} size={14} />
			{:else if data.kind === 'input'}
				<ArrowRightToLine size={14} />
			{:else if data.kind === 'result'}
				<Flag size={14} />
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
{/if}

<Handle type="target" position={Position.Left} isConnectable={false} class="!opacity-0" />
<Handle type="source" position={Position.Right} isConnectable={false} class="!opacity-0" />
