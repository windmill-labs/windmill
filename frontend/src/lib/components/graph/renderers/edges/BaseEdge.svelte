<script lang="ts">
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'
	import type { FlowModule } from '$lib/gen'
	import { getBezierPath, BaseEdge, type Position, EdgeLabelRenderer } from '@xyflow/svelte'
	import { ClipboardCopy } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import { getStraightLinePath } from '../utils'
	import InsertTriggerButton from '$lib/components/flows/map/InsertTriggerButton.svelte'

	export let sourceX: number
	export let sourceY: number
	export let sourcePosition: Position
	export let targetX: number
	export let targetY: number
	export let targetPosition: Position
	export let markerEnd: string | undefined = undefined

	export let data: {
		insertable: boolean
		modules: FlowModule[]
		moving: string | undefined
		eventHandlers: GraphEventHandlers
		index: number
		enableTrigger: boolean
		disableAi: boolean
	}

	$: [edgePath, labelX, labelY] = getBezierPath({
		sourceX,
		sourceY: targetY - sourceY > 100 ? targetY - 100 : sourceY,
		sourcePosition,
		targetX,
		targetY,
		targetPosition,
		curvature: 0.25
	})

	$: completeEdge =
		targetY - sourceY > 100
			? `${edgePath} ${getStraightLinePath({ sourceX, sourceY, targetY })}`
			: edgePath

	const { useDataflow } = getContext<{
		useDataflow: Writable<boolean | undefined>
	}>('FlowGraphContext')
</script>

<EdgeLabelRenderer>
	{#if data?.insertable && !$useDataflow && !data?.moving}
		<div
			class="edgeButtonContainer nodrag nopan"
			style:transform="translate(-50%, -50%) translate({labelX}px,{labelY}px)"
		>
			<InsertModuleButton
				disableAi={data.disableAi}
				index={data.index ?? 0}
				trigger={data.enableTrigger}
				modules={data?.modules ?? []}
				on:new={(e) => {
					data?.eventHandlers.insert({ modules: data.modules, index: data.index, detail: e.detail })
				}}
			/>
		</div>
		{#if data.enableTrigger}
			<div
				class="edgeButtonContainer nodrag nopan"
				style:transform="translate(100%, -50%) translate({labelX}px,{labelY}px)"
			>
				<InsertTriggerButton
					disableAi={data.disableAi}
					on:new={(e) => {
						data?.eventHandlers.insert({
							modules: data.modules,
							index: data.index,
							detail: e.detail
						})
					}}
					index={data?.index ?? 0}
					modules={data?.modules ?? []}
				/>
			</div>
		{/if}
	{/if}

	{#if data?.moving}
		<div
			class="edgeButtonContainer nodrag nopan"
			style:transform="translate(-50%, -50%) translate({labelX}px,{labelY}px)"
		>
			{#if data.moving}
				<button
					title="Paste module"
					on:click={() => {
						data.eventHandlers.insert({
							modules: data.modules,
							index: data.index
						})
					}}
					type="button"
					class="text-primary bg-surface border-[1px] mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
				>
					<ClipboardCopy class="m-[5px]" size={15} />
				</button>
			{/if}
		</div>
	{/if}
</EdgeLabelRenderer>

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
