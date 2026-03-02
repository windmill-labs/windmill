<script lang="ts">
	import InsertModulePopover from '$lib/components/flows/map/InsertModulePopover.svelte'
	import { getBezierPath, BaseEdge, type EdgeProps, EdgeLabel } from '@xyflow/svelte'
	import { CircleDot, Hourglass } from 'lucide-svelte'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import { getStraightLinePath } from '../utils'
	import { twMerge } from 'tailwind-merge'
	import { NODE_WITH_WRITE_ASSET_Y_OFFSET } from '../nodes/AssetNode.svelte'
	import { workspaceStore } from '$lib/stores'
	import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'
	import type { Job } from '$lib/gen'
	import type { GraphModuleState } from '../../model'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { getGraphContext } from '../../graphContext'

	const { useDataflow, showAssets, moveManager } = getGraphContext()

	let {
		id,
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
			branch: { rootId: string; branch: number } | undefined
			targetId: string
			eventHandlers: GraphEventHandlers
			index: number
			enableTrigger: boolean
			disableAi: boolean
			disableMoveIds: string[]
			flowModuleStates: Record<string, GraphModuleState> | undefined
			isOwner: boolean
			flowJob: Job | undefined
			suspendStatus?: Record<string, { job: Job; nb: number }>
			shouldOffsetInsertBtnDueToAssetNode?: boolean
		}
	} = $props()

	let [edgePath] = $derived(
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

	// TODO: this is a hack to show the waiting for events indicator on the edge a proper way would be to have a edge state
	// and handle the edge state in the graph builder
	let waitingForEvents = $derived(
		data?.flowModuleStates?.[data.targetId]?.type === 'WaitingForEvents' ||
			data?.flowModuleStates?.[`${data.sourceId}-v`]?.type === 'WaitingForEvents'
	)

	let suspendStatus: Record<string, { job: Job; nb: number }> | undefined = $derived(
		data?.suspendStatus
	)

	let centerY = $derived(
		sourceY +
			32 +
			(data.shouldOffsetInsertBtnDueToAssetNode && $showAssets ? NODE_WITH_WRITE_ASSET_Y_OFFSET : 0)
	)

	let isDragging = $derived(!!moveManager?.dragging)
	let draggedId = $derived(moveManager?.dragging?.moduleId)
	let isValidDropTarget = $derived(
		isDragging &&
			data?.insertable &&
			draggedId !== undefined &&
			!data.disableMoveIds?.includes(draggedId) &&
			data.sourceId !== draggedId &&
			data.targetId !== draggedId
	)
	let isNearestDrop = $derived(isValidDropTarget && moveManager?.nearestDropZone?.edgeId === id)
	let isAdjacentToDragged = $derived(
		isDragging && (data?.sourceId === draggedId || data?.targetId === draggedId)
	)

	// Register this edge's drop zone position with the drag manager so proximity
	// detection uses the actual xyflow-computed position rather than re-deriving it.
	$effect(() => {
		if (!data?.insertable || !moveManager) return

		const centerX = sourceX

		moveManager.registerDropZone(id, {
			sourceId: data.sourceId,
			targetId: data.targetId,
			branch: data.branch,
			index: data.index,
			disableMoveIds: data.disableMoveIds ?? [],
			centerX,
			centerY
		})

		return () => moveManager.unregisterDropZone(id)
	})
</script>

<EdgeLabel x={sourceX} y={centerY} class="base-edge" style="">
	{#if waitingForEvents && data.flowJob && data.flowJob.type === 'QueuedJob'}
		<div
			class="px-2 py-0.5 rounded-md bg-surface shadow-md text-violet-700 dark:text-violet-400 text-xs flex items-center gap-1"
		>
			<Hourglass size={12} />
			<div class="flex">
				<span class="dot">.</span>
				<span class="dot">.</span>
				<span class="dot">.</span>
			</div>
		</div>
		<div
			class={'fixed top-1/2 -translate-y-1/2 left-[170px] h-fit w-fit rounded-md bg-surface flex items-center justify-center p-2 ml-2 shadow-md'}
		>
			{#if data?.flowJob && data.flowJob.flow_status?.modules?.[data.flowJob.flow_status?.step]?.type === 'WaitingForEvents'}
				<FlowStatusWaitingForEvents
					job={data.flowJob}
					workspaceId={$workspaceStore!}
					isOwner={data.isOwner}
					light
				/>
			{:else if suspendStatus && Object.keys(suspendStatus).length > 0}
				<div class="flex gap-2 flex-col">
					{#each Object.values(suspendStatus) as suspendCount (suspendCount.job.id)}
						<FlowStatusWaitingForEvents
							job={suspendCount.job}
							workspaceId={$workspaceStore!}
							isOwner={data.isOwner}
							light
						/>
					{/each}
				</div>
			{/if}
		</div>
	{:else if isDragging && isValidDropTarget}
		<div class="edgeButtonContainer nodrag nopan" style:transform="translate(-50%, -50%)">
			<div class="relative flex items-center justify-center" style="width: 275px; height: 20px;">
				{#if isNearestDrop}
					<div class="absolute inset-0 rounded-md bg-accent/5 transition-opacity duration-150"
					></div>
				{/if}
				{@render dropTargetIndicator(isNearestDrop)}
			</div>
		</div>
	{:else if data?.insertable && !$useDataflow && !moveManager?.movingModuleId && !isDragging}
		<div
			class={twMerge('edgeButtonContainer nodrag nopan top-0')}
			style:transform="translate(-50%, -50%)"
		>
			<!-- <pre class="text-2xs">A{JSON.stringify(data.branch)}, {data.sourceId}, {data.targetId}</pre> -->
			<!-- {data.targetId} B -->
			<InsertModulePopover
				disableAi={data.disableAi}
				allowTrigger={data.index == 0}
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
				gutter={0}
			>
				{#snippet trigger()}
					<InsertModuleButton title={`Add step`} id={`flow-editor-add-step-${data.index ?? 0}`} />
				{/snippet}
			</InsertModulePopover>
		</div>
	{/if}

	{#if moveManager?.movingModuleId && data?.insertable}
		<div class="edgeButtonContainer nodrag nopan" style:transform="translate(-50%, -50%)">
			{#if !data.disableMoveIds?.includes(moveManager.movingModuleId)}
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
					class="group relative flex items-center justify-center"
					style="width: 275px; height: 20px;"
				>
					<div
						class="absolute inset-0 rounded-md bg-accent/5 opacity-0 group-hover:opacity-100 transition-opacity duration-150"
					></div>
					{@render dropTargetIndicator(false)}
				</button>
			{/if}
		</div>
	{/if}
</EdgeLabel>

{#snippet dropTargetIndicator(highlighted: boolean)}
	<div
		class={twMerge(
			'w-[20px] h-[20px] flex items-center justify-center rounded-md bg-surface-secondary transition-all duration-150 group-hover:text-accent',
			highlighted ? 'text-accent' : 'text-primary'
		)}
	>
		<CircleDot size={12} />
	</div>
{/snippet}

<BaseEdge
	path={completeEdge}
	{markerEnd}
	class={$useDataflow ? 'hidden' : isAdjacentToDragged ? 'opacity-30' : ''}
	interactionWidth={0}
	style={undefined}
	label={undefined}
	labelStyle={undefined}
/>

<style>
	.edgeButtonContainer {
		position: absolute;
		font-size: 12pt;
		/* everything inside EdgeLabelRenderer has no pointer events by default */
		/* if you have an interactive element, set pointer-events: all */
		pointer-events: all;
	}

	.dot {
		opacity: 0;
		animation: dotFade 1.5s infinite;
		letter-spacing: 1px;
	}

	.dot:nth-child(2) {
		animation-delay: 0.5s;
	}

	.dot:nth-child(3) {
		animation-delay: 1s;
	}

	@keyframes dotFade {
		0%,
		100% {
			opacity: 0;
		}
		50% {
			opacity: 1;
		}
	}
</style>
