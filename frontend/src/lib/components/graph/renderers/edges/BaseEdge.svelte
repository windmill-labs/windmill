<script lang="ts">
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { getBezierPath, BaseEdge, type EdgeProps, EdgeLabel } from '@xyflow/svelte'
	import { ClipboardCopy } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import { getStraightLinePath } from '../utils'
	import { twMerge } from 'tailwind-merge'

	const { useDataflow } = getContext<{
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext')

	let {
		// id,
		sourceX,
		sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		markerEnd,
		// style,
		data
	}: EdgeProps & {
		data: {
			insertable: boolean
			sourceId: string
			branch: number | undefined
			targetId: string
			moving: string | undefined
			eventHandlers: GraphEventHandlers
			index: number
			enableTrigger: boolean
			disableAi: boolean
			disableMoveIds: string[]
		}
	} = $props()

	let [edgePath, labelX, labelY] = $derived(
		getBezierPath({
			sourceX,
			sourceY: targetY - sourceY > 100 ? targetY - 100 : sourceY,
			sourcePosition,
			targetX,
			targetY,
			targetPosition,
			curvature: 0.25
		})
	)

	let completeEdge = $derived(
		targetY - sourceY > 100
			? `${edgePath} ${getStraightLinePath({ sourceX, sourceY, targetY })}`
			: edgePath
	)
</script>

<EdgeLabel x={labelX} y={labelY} class="base-edge" style="">
	{#if data?.insertable && !$useDataflow && !data?.moving}
		<div
			class={twMerge('edgeButtonContainer nodrag nopan top-0')}
			style:transform="translate(-50%, -50%)"
		>
			<!-- <pre class="text-2xs">A{JSON.stringify(data.branch)}, {data.sourceId}, {data.targetId}</pre> -->
			<!-- {data.targetId} B -->
			<InsertModuleButton
				index={data.index ?? 0}
				on:new={(e) => {
					data?.eventHandlers.insert({
						sourceId: data.sourceId,
						targetId: data.targetId,
						branch: data.branch,
						index: data.index,
						kind: e.detail.kind,
						inlineScript: e.detail.inlineScript
					})
				}}
				on:pickScript={(e) => {
					// console.log('pickScript', e)
					data?.eventHandlers.insert({
						sourceId: data.sourceId,
						targetId: data.targetId,
						branch: data.branch,
						index: data.index,
						script: e.detail,
						kind: e.detail.kind
					})
				}}
				on:pickFlow={(e) => {
					// console.log('pickFlow', e)
					data?.eventHandlers.insert({
						sourceId: data.sourceId,
						targetId: data.targetId,
						branch: data.branch,
						index: data.index,
						flow: e.detail
					})
				}}
			/>
		</div>
	{/if}

	{#if data?.moving}
		<div class="edgeButtonContainer nodrag nopan" style:transform="translate(-50%, -50%)">
			{#if data.moving && !data.disableMoveIds?.includes(data.moving)}
				<button
					title="Paste module"
					onclick={() => {
						data.eventHandlers.insert({
							branch: data.branch,
							sourceId: data.sourceId,
							targetId: data.targetId,
							index: data.index
						})
					}}
					type="button"
					class={twMerge(
						'w-6 h-6 flex items-center justify-center',
						'border border-gray-300 dark:border-gray-500',
						'text-primary text-sm',
						'bg-surface focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected rounded-full '
					)}
				>
					<ClipboardCopy size={14} />
				</button>
			{/if}
		</div>
	{/if}
</EdgeLabel>

<BaseEdge path={completeEdge} {markerEnd} class={$useDataflow ? 'hidden' : ''} />

<style>
	.edgeButtonContainer {
		position: absolute;
		font-size: 12pt;
		/* everything inside EdgeLabelRenderer has no pointer events by default */
		/* if you have an interactive element, set pointer-events: all */
		pointer-events: all;
	}
</style>
