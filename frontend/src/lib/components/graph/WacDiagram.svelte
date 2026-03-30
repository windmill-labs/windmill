<script lang="ts">
	import { SvelteFlow, SvelteFlowProvider, Controls, type Node, type Edge } from '@xyflow/svelte'
	import { parseWacDag, type WacWorkflowDag } from '$lib/infer'
	import { dagToXyflow } from './wacDagLayout'
	import WacStepNode from './renderers/nodes/WacStepNode.svelte'
	import WacControlNode from './renderers/nodes/WacControlNode.svelte'
	import WacEdge from './renderers/edges/WacEdge.svelte'
	import { AlertTriangle, Workflow } from 'lucide-svelte'

	interface Props {
		code: string
		language: string
	}

	let { code, language }: Props = $props()

	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])
	let errors = $state<{ message: string; line: number }[]>([])
	let empty = $state(true)

	let debounceTimer: ReturnType<typeof setTimeout> | undefined

	$effect(() => {
		const _code = code
		const _lang = language

		clearTimeout(debounceTimer)
		debounceTimer = setTimeout(async () => {
			const result = await parseWacDag(_code, _lang)
			if (result === null) {
				nodes = []
				edges = []
				errors = []
				empty = true
			} else if ('errors' in result) {
				nodes = []
				edges = []
				errors = result.errors
				empty = false
			} else {
				const dag = result as WacWorkflowDag
				const layout = dagToXyflow(dag)
				nodes = layout.nodes
				edges = layout.edges
				errors = []
				empty = dag.nodes.length === 0
			}
		}, 300)

		return () => clearTimeout(debounceTimer)
	})

	const nodeTypes = {
		wacStep: WacStepNode,
		wacControl: WacControlNode
	} as any

	const edgeTypes = {
		wacEdge: WacEdge
	} as any

	const proOptions = { hideAttribution: true }
</script>

<div class="w-full h-full">
	{#if errors.length > 0}
		<div class="p-4 flex flex-col gap-2">
			{#each errors as error (error.line)}
				<div class="flex items-start gap-2 text-xs text-red-500">
					<AlertTriangle size={14} class="shrink-0 mt-0.5" />
					<span>
						{#if error.line > 0}
							<span class="font-mono">L{error.line}:</span>
						{/if}
						{error.message}
					</span>
				</div>
			{/each}
		</div>
	{:else if empty}
		<div class="p-4 flex flex-col items-center justify-center h-full text-tertiary text-xs gap-2">
			<Workflow size={24} />
			<span>No workflow diagram</span>
		</div>
	{:else}
		<SvelteFlowProvider>
			<SvelteFlow
				{nodes}
				{edges}
				{nodeTypes}
				{edgeTypes}
				fitView
				fitViewOptions={{ padding: 0.2 }}
				nodesDraggable={false}
				elementsSelectable={false}
				panOnDrag={true}
				zoomOnScroll={true}
				zoomOnPinch={true}
				zoomOnDoubleClick={false}
				preventScrolling={true}
				{proOptions}
				style="background: transparent;"
			>
				<Controls position="top-right" orientation="horizontal" showLock={false} />
			</SvelteFlow>
		</SvelteFlowProvider>
	{/if}
</div>
