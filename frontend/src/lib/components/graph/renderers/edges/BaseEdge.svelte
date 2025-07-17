<script lang="ts">
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import { getBezierPath, BaseEdge, type EdgeProps, EdgeLabel } from '@xyflow/svelte'
	import { ClipboardCopy, Hourglass } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder.svelte'
	import { getStraightLinePath } from '../utils'
	import { twMerge } from 'tailwind-merge'
	import { type FlowGraphAssetContext } from '$lib/components/flows/types'
	import {
		assetDisplaysAsOutputInFlowGraph,
		NODE_WITH_WRITE_ASSET_Y_OFFSET
	} from '../nodes/AssetNode.svelte'
	import { workspaceStore } from '$lib/stores'
	import FlowStatusWaitingForEvents from '$lib/components/FlowStatusWaitingForEvents.svelte'
	import type { Job } from '$lib/gen'
	import type { GraphModuleState } from '../../model'

	const { useDataflow } = getContext<{
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext')

	const flowGraphAssetCtx = getContext<FlowGraphAssetContext | undefined>('FlowGraphAssetContext')

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
			flowModuleStates: Record<string, GraphModuleState> | undefined
			isOwner: boolean
			flowJob: Job | undefined
			suspendStatus?: Writable<Record<string, { job: Job; nb: number }>>
		}
	} = $props()

	const shouldOffsetInsertButtonDueToAssetNode = flowGraphAssetCtx?.val.assetsMap?.[
		data.sourceId
	]?.some(assetDisplaysAsOutputInFlowGraph)

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

	const suspendStatus: Writable<Record<string, { job: Job; nb: number }>> | undefined = $derived(
		data?.suspendStatus
	)
</script>

<EdgeLabel
	x={sourceX}
	y={sourceY + 28 + (shouldOffsetInsertButtonDueToAssetNode ? NODE_WITH_WRITE_ASSET_Y_OFFSET : 0)}
	class="base-edge"
	style=""
>
	{#if data?.insertable && !$useDataflow && !data?.moving && !waitingForEvents}
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
			{:else if $suspendStatus && Object.keys($suspendStatus).length > 0}
				<div class="flex gap-2 flex-col">
					{#each Object.values($suspendStatus) as suspendCount (suspendCount.job.id)}
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
