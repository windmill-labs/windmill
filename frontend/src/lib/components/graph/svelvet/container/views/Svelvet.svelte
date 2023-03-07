<script lang="ts">
	const pkStringGenerator = () => (Math.random() + 1).toString(36).substring(7)
	import type { UserEdgeType, UserNodeType } from '../../types/types'

	import {
		createStoreEmpty,
		populateSvelvetStoreFromUserInput
	} from '../../store/controllers/storeApi'
	import { afterUpdate, onMount } from 'svelte'
	import GraphView from './GraphView.svelte'
	import { sanitizeCanvasOptions, sanitizeUserNodesAndEdges } from '../controllers/middleware'

	export let nodes: UserNodeType[] // TODO: update type to make possible user id being a number
	export let edges: UserEdgeType[] // TODO: update type to make possible user id being a number
	export let bgColor = '#ffffff' // this is used to set the background color of the the Svelvet canvas
	export let minimap = false
	export let width: number = 600
	export let height: number = 600
	export let background: boolean = true
	export let movement: boolean = true
	export let canvasId: string = pkStringGenerator()
	export let snap: boolean = false
	export let snapTo: number = 30
	export let nodeCreate: boolean = false
	export let initialZoom = 3
	export let initialLocation = { x: 0, y: 0 }
	export let boundary = false
	export let collapsible = false
	export let shareable: boolean = false // used for "importExport" feature
	export let locked: boolean = false // if true, node movement is disabled
	export let editable: boolean = false
	export let resizable: boolean = true
	export let highlightEdges: boolean = true
	export let scroll: boolean = false

	// generates a unique string for each svelvet component's unique store instance
	// creates a store that uses the unique sting as the key to create and look up the corresponding store
	// this way we can have multiple Svelvet Components on the same page and prevent overlap of information
	const store = createStoreEmpty(canvasId)
	// stores (state) within stores, so that we cannot access values from everywhere
	//   const { widthStore, heightStore, nodesStore, derivedEdges } = svelvetStore;

	// sets the state of the store to the values passed in from the Svelvet Component on initial render
	onMount(() => {
		// sanitize user input
		let output = sanitizeUserNodesAndEdges(nodes, edges)
		const userNodes = output['userNodes']
		const userEdges = output['userEdges']

		// set canvas related stores. you need to do this before setting node/edge related stores because
		// initializing nodes/edges might read relevant options from the store.
		store.widthStore.set(width)
		store.heightStore.set(height)
		store.backgroundStore.set(background)
		store.movementStore.set(movement)
		const optionsObj = { snap, snapTo } // TODO: rename to snap
		store.options.set(optionsObj) //
		store.nodeCreate.set(nodeCreate)
		store.boundary.set(boundary)
		store.collapsibleOption.set(collapsible)
		store.lockedOption.set(locked)
		store.editableOption.set(editable)
		store.resizableOption.set(resizable)
		store.highlightEdgesOption.set(highlightEdges)

		// make sure that all canvas options are compatible
		sanitizeCanvasOptions(store)
		// set node/edge related stores
		populateSvelvetStoreFromUserInput(canvasId, userNodes, userEdges)
	})
	// // enables data reactivity. TODO: this needs to be added back in
	// Probably need to use findStore, not create store
	afterUpdate(() => {
		// sanitize user input
		let output = sanitizeUserNodesAndEdges(nodes, edges)
		const userNodes = output['userNodes']
		const userEdges = output['userEdges']

		// set canvas related stores. you need to do this before setting node/edge related stores because
		// initializing nodes/edges might read relevant options from the store.
		store.widthStore.set(width)
		store.heightStore.set(height)
		store.backgroundStore.set(background)
		store.movementStore.set(movement)
		const optionsObj = { snap, snapTo } // TODO: rename to snap
		store.options.set(optionsObj) //
		store.nodeCreate.set(nodeCreate)
		store.boundary.set(boundary)
		store.collapsibleOption.set(collapsible)
		store.lockedOption.set(locked)
		store.editableOption.set(editable)
		store.resizableOption.set(resizable)
		store.highlightEdgesOption.set(highlightEdges)

		// make sure that all canvas options are compatible
		sanitizeCanvasOptions(store)
		// set node/edge related stores
		populateSvelvetStoreFromUserInput(canvasId, userNodes, userEdges)
	})
</script>

<!-- Now that a store has been created from the initial nodes and initial edges we drill props from the store down to the D3 GraphView along with the unique key -->
<div
	class="Svelvet"
	style={`width: ${width}px; height: ${height}px; background-color: ${bgColor};`}
>
	<GraphView
		{scroll}
		{canvasId}
		{width}
		{height}
		{initialLocation}
		{initialZoom}
		{boundary}
		{minimap}
	/>
</div>

<style>
	.Svelvet {
		position: relative;
		overflow: hidden;
		display: grid;
		font-family: 'Segoe UI', sans-serif;
	}
</style>
