<script lang="ts">
	import { SvelteFlow, Controls, SvelteFlowProvider, type Node } from '@xyflow/svelte'
	import '@xyflow/svelte/dist/base.css'
	import { SvelteSet } from 'svelte/reactivity'
	import { Loader2 } from 'lucide-svelte'
	import Alert from '../common/alert/Alert.svelte'
	import GraphZoomControls from '../graph/GraphZoomControls.svelte'
	import DbTableNode from './DbTableNode.svelte'
	import DiagramAutoFit from './DiagramAutoFit.svelte'
	import { buildRelationIndex, type DbRelation } from '../dbRelations'
	import { DbDiagramHighlight, setDbDiagramHighlight } from './dbDiagramHighlight.svelte'
	import { buildDiagramTables, cardHeight, layoutTables, CARD_WIDTH } from './dbDiagramModel'
	import type { DBSchema } from '$lib/stores'
	import type { ColumnDef } from '../apps/components/display/dbtable/utils'
	import type { SelectedTable } from '../DBManager.svelte'

	interface Props {
		dbSchema: DBSchema
		colDefs: Record<string, ColumnDef[]> | undefined
		selectedTables: SelectedTable[]
		relations: DbRelation[]
		loading?: boolean
		error?: string | undefined
		/** Opens a table in the data view. */
		onOpenTable?: (table: SelectedTable) => void
	}

	let {
		dbSchema,
		colDefs,
		selectedTables,
		relations,
		loading = false,
		error = undefined,
		onOpenTable
	}: Props = $props()

	let index = $derived(buildRelationIndex(relations))
	const highlight = new DbDiagramHighlight(() => index)
	setDbDiagramHighlight(highlight)

	let expanded = new SvelteSet<string>()

	let tables = $derived(buildDiagramTables(dbSchema, colDefs, selectedTables))

	// Laid out from scratch on every change to the selection or to a card's
	// height. There are no edges to keep stable, so a fresh packing reads better
	// than leaving holes where deselected tables were.
	let nodes = $state.raw<Node[]>([])
	$effect(() => {
		const positions = layoutTables(tables, expanded)
		nodes = tables.map((table) => ({
			id: table.key,
			type: 'dbTable',
			position: positions[table.key],
			width: CARD_WIDTH,
			height: cardHeight(table, expanded.has(table.key)),
			data: {
				table,
				expanded: expanded.has(table.key),
				index,
				onToggleExpand: () => {
					if (!expanded.delete(table.key)) expanded.add(table.key)
				},
				onOpenTable: () => onOpenTable?.({ schema: table.schema, table: table.table })
			}
		}))
	})

	let fitKey = $derived(tables.map((t) => t.key).join('\n'))

	const nodeTypes = { dbTable: DbTableNode }
</script>

<div class="h-full w-full relative">
	{#if error}
		<div class="absolute inset-x-0 top-0 z-10 p-2">
			<Alert type="error" title="Could not load the relationships" size="xs">{error}</Alert>
		</div>
	{/if}
	{#if tables.length === 0}
		<div class="h-full w-full center-center">
			<span class="text-sm text-hint">Check tables in the sidebar to draw them here</span>
		</div>
	{:else}
		<SvelteFlowProvider>
			<SvelteFlow
				bind:nodes
				edges={[]}
				{nodeTypes}
				minZoom={0.1}
				maxZoom={1.6}
				proOptions={{ hideAttribution: true }}
				elementsSelectable={false}
				zoomOnDoubleClick={false}
				deleteKey={null}
				onpaneclick={() => highlight.clear()}
			>
				<div class="absolute inset-0 !bg-surface-secondary h-full"></div>
				<DiagramAutoFit key={fitKey} />
				<Controls
					class="wm-db-diagram-controls"
					position="bottom-right"
					orientation="horizontal"
					showLock={false}
					showZoom={false}
					showFitView={false}
				>
					<GraphZoomControls />
				</Controls>
			</SvelteFlow>
		</SvelteFlowProvider>
	{/if}
	{#if loading}
		<div
			class="absolute top-2 left-2 z-10 flex items-center gap-2 text-xs text-tertiary bg-surface-secondary rounded px-2 py-1"
		>
			<Loader2 size={12} class="animate-spin" />
			Loading relationships
		</div>
	{/if}
</div>

<style lang="postcss">
	/* xy-flow's own control rules are nested and match ours exactly, so scoping by
	   the class passed to <Controls> outranks them rather than racing load order. */
	:global(.svelte-flow__controls.wm-db-diagram-controls) {
		@apply overflow-hidden rounded-md border border-border-light bg-surface;
		box-shadow: none;
	}
	:global(.wm-db-diagram-controls .svelte-flow__controls-button) {
		@apply bg-surface text-primary border-r border-border-light;
		width: 32px;
		height: 30px;
		padding: 8px;
	}
	:global(.wm-db-diagram-controls .svelte-flow__controls-button:last-child) {
		@apply border-r-0;
	}
	:global(.wm-db-diagram-controls .svelte-flow__controls-button:hover) {
		@apply bg-surface-hover;
	}
	/* The glyphs are lucide, so undo xy-flow's `fill: currentColor` and its 12px cap. */
	:global(.wm-db-diagram-controls .svelte-flow__controls-button svg) {
		max-width: 16px;
		max-height: 16px;
		fill: none;
		stroke: currentColor;
	}
</style>
