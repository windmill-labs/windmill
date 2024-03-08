<script lang="ts">
	import { pointer, select, selectAll } from 'd3-selection'
	import { zoom, zoomIdentity, zoomTransform } from 'd3-zoom'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import SimpleBezierEdge from '../../edges/views/Edges/SimpleBezierEdge.svelte'
	import SmoothStepEdge from '../../edges/views/Edges/SmoothStepEdge.svelte'
	import StepEdge from '../../edges/views/Edges/StepEdge.svelte'

	import Node from '../../nodes/views/Node.svelte'

	import { determineD3Instance } from '../..//d3/controllers/d3'
	import { findStore } from '../../store/models/store'

	import { Expand, Minus, Plus } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { fade } from 'svelte/transition'

	//these are typscripted as any, however they have been transformed inside of store.ts
	export let canvasId: string
	export let width: number
	export let height: number

	export let boundary = false
	export let scroll = false

	export let dataflow = false
	export let download = false
	export let showDataflowToggle: boolean = false

	// here we lookup the store using the unique key
	const store = findStore(canvasId)
	const {
		edgesStore,
		nodesStore,
		nodeSelected,
		backgroundStore,
		movementStore,
		widthStore,
		heightStore,
		d3Scale
	} = store
	$: nodes = Object.values($nodesStore)
	$: edges = Object.values($edgesStore)

	// declaring the grid and dot size for d3's transformations and zoom
	const gridSize = 15
	const dotSize = 10

	const dispatch = createEventDispatcher()

	// leveraging d3 library to zoom/pan
	let d3 = {
		zoom,
		zoomTransform,
		zoomIdentity,
		select,
		selectAll,
		pointer
	}

	let d3Zoom = determineD3Instance(
		boundary,
		d3,
		nodeSelected,
		width,
		height,
		movementStore,
		backgroundStore,
		gridSize,
		dotSize,
		canvasId,
		d3Scale,
		handleZoom
	)

	// d3Translate is used for the minimap
	onMount(() => {
		// actualizes the d3 instance

		const nodes = d3.select(`#zoomable-${canvasId}`).call(d3Zoom).on('dblclick.zoom', null)

		if (!scroll) {
			;[nodes].forEach((d3Instance) => {
				d3Instance
					.on('wheel.zoom', null)
					.on('mousewheel.zoom', null)
					.on('mousemove.zoom', null)
					.on('DOMMouseScroll.zoom', null)
			})
		}

		d3.select(`#zoom_in_${canvasId}`).on('click', function () {
			try {
				// @ts-ignore
				d3Zoom.scaleBy(nodes.transition().duration(250), 1.4)
			} catch (e) {
				console.log('error', e)
			}
		})
		d3.select(`#zoom_out_${canvasId}`).on('click', function () {
			try {
				// @ts-ignore
				d3Zoom.scaleBy(nodes.transition().duration(250), 0.714)
			} catch (e) {
				console.log('error', e)
			}
		})
	})

	function handleZoom(e) {
		if (!$movementStore) return
		//add a store that contains the current value of the d3-zoom's scale to be used in onMouseMove function
		d3Scale.set(e.transform.k)
		// transform 'g' SVG elements (edge, edge text, edge anchor)
		d3.select(`.Edges-${canvasId} g`).attr('transform', e.transform)
		// transform div elements (nodes)
		//@ts-ignore
		let transform = d3.zoomTransform(this)
		store.d3ZoomParameters.set({ ...transform }) // record x,y position of pan, and zoom level
		// selects and transforms all node divs from class 'Node' and performs transformation
		d3.select(`.Node-${canvasId}`)
			.style(
				'transform',
				'translate(' + transform.x + 'px,' + transform.y + 'px) scale(' + transform.k + ')'
			)
			.style('transform-origin', '0 0')
	}

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	$: {
		if ($copilotCurrentStepStore !== undefined) {
			// reset zoom
			d3.select(`#zoomable-${canvasId}`).call(d3Zoom.transform, d3.zoomIdentity)
		}
	}
</script>

<div class="zoomable" id={`zoomable-${canvasId}`}>
	<!-- This is the container that holds GraphView and we have disabled right click functionality to prevent a sticking behavior -->
	<div id="graphview-container">
		<div class={`Nodes Nodes-${canvasId}`} on:contextmenu|preventDefault>
			<!-- This container is transformed by d3zoom -->
			<div class={`Node Node-${canvasId}`}>
				{#if $copilotCurrentStepStore !== undefined}
					<div transition:fade class="fixed inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0" />
				{/if}
				{#each nodes as node}
					{#if node.data.html}
						<Node {node} {canvasId}>{@html node.data.html}</Node>
					{:else if node.data.custom}
						<Node isCustom {node} {canvasId}>
							<svelte:component
								this={node.data.custom.component}
								on:new={(e) => node?.data?.custom?.cb?.('new', e.detail)}
								on:delete={(e) => node?.data?.custom?.cb?.('delete', e.detail)}
								on:select={(e) => node?.data?.custom?.cb?.('select', e.detail)}
								on:insert={(e) => node?.data?.custom?.cb?.('insert', e.detail)}
								on:newBranch={(e) => node?.data?.custom?.cb?.('newBranch', e.detail)}
								on:deleteBranch={(e) => node?.data?.custom?.cb?.('deleteBranch', e.detail)}
								on:move={(e) => node?.data?.custom?.cb?.('move', e.detail)}
								on:nodeInsert={(e) => node?.data?.custom?.cb?.('nodeInsert', e.detail)}
								on:addBranch={(e) => node?.data?.custom?.cb?.('addBranch', e.detail)}
								on:removeBranch={(e) => node?.data?.custom?.cb?.('removeBranch', e.detail)}
								{...node.data.custom.props}
							/>
						</Node>
					{:else}
						<Node {node} {canvasId}>{node.data.label}</Node>
					{/if}
				{/each}
			</div>
		</div>
	</div>
	<!-- rendering dots on the background depending on the zoom level -->
	<svg
		class={`Edges Edges-${canvasId}`}
		viewBox="0 0 {$widthStore} {$heightStore}"
		on:contextmenu|preventDefault
	>
		<defs>
			<pattern
				id={`background-${canvasId}`}
				x="0"
				y="0"
				width={gridSize}
				height={gridSize}
				patternUnits="userSpaceOnUse"
			>
				<circle
					id="dot"
					cx={gridSize / 2 - dotSize / 2}
					cy={gridSize / 2 - dotSize / 2}
					r="0.5"
					style="fill: gray"
				/>
			</pattern>
		</defs>

		<!-- <g> tag defines which edge type to render depending on properties of edge object -->
		<g>
			{#each edges as edge}
				{#if edge.type === 'smoothstep'}
					<SmoothStepEdge {edge} {canvasId} />
				{:else if edge.type === 'step'}
					<StepEdge {edge} {canvasId} />
				{:else}
					<SimpleBezierEdge edgeId={edge.id} {canvasId} />
				{/if}
			{/each}

			<!-- {#each filteredAnchors as anchor} -->
			<!-- note that these are SVG -->
			<!-- <EdgeAnchor x={anchor.positionX} y={anchor.positionY} /> -->
			<!-- {/each} -->
		</g>
	</svg>
</div>
{#if showDataflowToggle}
	<div id="dataflow_toggle">
		<Toggle
			disabled={$copilotCurrentStepStore !== undefined}
			textClass="!text-secondary"
			size="xs"
			bind:checked={dataflow}
			options={{
				right: 'dataflow'
			}}
		/>
	</div>
{/if}

<div id="buttons">
	<button
		title="Zoom In"
		id={`zoom_in_${canvasId}`}
		class={'relative overflow-hidden ' +
			($copilotCurrentStepStore !== undefined
				? '!bg-gray-400/20 text-black/20 border-gray-400/20'
				: '')}
	>
		<Plus size="14" class="flex justify-start m-1" />
	</button>
	<button
		title="Zoom Out"
		id={`zoom_out_${canvasId}`}
		class={'relative overflow-hidden ' +
			($copilotCurrentStepStore !== undefined
				? '!bg-gray-400/20 text-black/20 border-gray-400/20'
				: '')}
	>
		<Minus size="14" class="flex justify-start m-1" />
	</button>

	{#if download}
		<button on:click={() => dispatch('expand')}>
			<Expand size="14" class="flex justify-start m-1" />
		</button>
	{/if}
</div>

<style>
	#dataflow_toggle {
		position: absolute;
		top: 3px;
		left: 8px;
	}
	#buttons {
		position: absolute;
		top: 8px;
		right: 8px;
		z-index: 20;
	}

	#buttons > button {
		border-radius: 4px;
		@apply bg-surface;
		@apply border;
		transition: all;
	}

	.Nodes {
		position: absolute;
		width: 100%;
		height: 100%;
	}
	.Node {
		color: black; /* remove this once color is set to default via types */
		width: 100%;
		height: 100%;
	}
	#graphview-container {
		pointer-events: none;
	}
	.pointer-events-auto {
		pointer-events: auto;
	}
</style>
