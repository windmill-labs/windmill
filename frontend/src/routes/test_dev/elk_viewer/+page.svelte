<script lang="ts">
	import '@xyflow/svelte/dist/base.css'
	import {
		SvelteFlow,
		SvelteFlowProvider,
		type Node,
		type Edge,
		Controls
	} from '@xyflow/svelte'

	let jsonInput = $state(SAMPLE_ELK_RESULT)
	let error = $state('')
	let nodes = $state.raw<Node[]>([])
	let edges = $state.raw<Edge[]>([])

	/**
	 * Parse an ELK layout result using SvelteFlow's parentId system.
	 *
	 * Approach:
	 * - Compound nodes (have children) → rendered as group wrappers (dashed border)
	 *   Position is relative to their own parent (or absolute if at root).
	 * - Leaf nodes → rendered as regular nodes with parentId pointing to their
	 *   compound parent. Position is relative to parent (ELK already gives relative coords).
	 * - Nodes must be ordered: parents before children.
	 */
	function parseElkResult(elkResult: any) {
		const svelteNodes: Node[] = []
		const svelteEdges: Edge[] = []

		function walkNodes(elkNode: any, parentId: string | undefined) {
			for (const child of elkNode.children ?? []) {
				const isCompound = child.children?.length > 0

				const node: Node = {
					id: child.id,
					position: { x: child.x ?? 0, y: child.y ?? 0 },
					data: { label: child.id },
					type: 'default',
					...(parentId ? { parentId } : {}),
					...(isCompound
						? {
								style: `width: ${child.width}px; height: ${child.height}px; background: rgba(100,149,237,0.06); border: 1px dashed #6495ed; border-radius: 8px; font-size: 10px; color: #6495ed; padding: 4px;`,
								// Prevent group from intercepting pointer events on children
								selectable: false
							}
						: {
								style: `width: ${child.width ?? 275}px; height: ${child.height ?? 34}px; font-size: 11px;`
							})
				}

				// Parent (compound) nodes must come before their children
				svelteNodes.push(node)

				if (isCompound) {
					// Recurse: children will get parentId = this compound's id
					walkNodes(child, child.id)
				}
			}
		}

		function walkEdges(elkNode: any) {
			for (const edge of elkNode.edges ?? []) {
				svelteEdges.push({
					id: edge.id,
					source: edge.sources?.[0] ?? '',
					target: edge.targets?.[0] ?? '',
					type: 'default'
				})
			}
			for (const child of elkNode.children ?? []) {
				walkEdges(child)
			}
		}

		walkNodes(elkResult, undefined)
		walkEdges(elkResult)

		return { svelteNodes, svelteEdges }
	}

	function renderFromJson() {
		try {
			const parsed = JSON.parse(jsonInput)
			const { svelteNodes, svelteEdges } = parseElkResult(parsed)
			nodes = svelteNodes
			edges = svelteEdges
			error = ''
		} catch (e: any) {
			error = e.message
		}
	}

	$effect(() => {
		renderFromJson()
	})

	const proOptions = { hideAttribution: true }
</script>

<div class="flex h-screen w-screen">
	<!-- Left: JSON editor -->
	<div class="w-[500px] flex flex-col border-r border-gray-300 bg-white">
		<div class="flex items-center justify-between p-2 border-b border-gray-200 bg-gray-50">
			<h2 class="text-sm font-bold">ELK Result JSON</h2>
			<button
				class="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
				onclick={renderFromJson}
			>
				Render
			</button>
		</div>
		{#if error}
			<div class="p-2 text-xs text-red-600 bg-red-50 border-b border-red-200">{error}</div>
		{/if}
		<textarea
			class="flex-1 p-2 font-mono text-xs resize-none border-0 outline-none"
			bind:value={jsonInput}
			spellcheck="false"
		></textarea>
	</div>

	<!-- Right: SvelteFlow canvas -->
	<div class="flex-1 relative">
		<SvelteFlowProvider>
			<SvelteFlow
				{nodes}
				{edges}
				fitView
				{proOptions}
				minZoom={0.1}
				maxZoom={3}
				nodesDraggable={false}
			>
				<Controls />
			</SvelteFlow>
		</SvelteFlowProvider>
	</div>
</div>

<style>
	:global(.svelte-flow) {
		background: #fafafa;
	}
</style>

<script module lang="ts">
	/**
	 * Sample ELK result demonstrating the wrapper approach:
	 *
	 * - `a` is a regular leaf node (the branchAll step), rendered before the wrapper
	 * - `a__group` is the invisible compound wrapper (holds all branch lanes)
	 * - Each `a-branch-X` is a compound lane (dashed border for debug)
	 *   with the branch-start label as its FIRST child node
	 * - Remaining nodes (b, c, d, ...) are leaf children inside their lane
	 *
	 * Edges at root: Trigger→Input→a→a__group→a-end→Result
	 * Edges inside lanes: branch-label→content nodes
	 */
	const SAMPLE_ELK_RESULT = JSON.stringify(
		{
			id: 'root',
			layoutOptions: {
				'elk.algorithm': 'layered',
				'elk.direction': 'DOWN'
			},
			children: [
				{ id: 'Trigger', width: 275, height: 34, x: 385, y: 12 },
				{ id: 'Input', width: 275, height: 34, x: 385, y: 108 },
				// `a` = branchAll step, rendered as a regular leaf node
				{ id: 'a', width: 275, height: 34, x: 385, y: 204 },
				// `a__group` = invisible wrapper compound for the branch lanes
				// width/height must fully contain all children (no overflow)
				// max child right = 710+299 = 1009, +12 padding = 1021
				// max child bottom = 12+346 = 358, +12 padding = 370
				{
					id: 'a__group',
					x: 12,
					y: 300,
					width: 1021,
					height: 370,
					children: [
						// Lane 0: compound with branch-start label as first child
						{
							id: 'a-branch-0',
							x: 12,
							y: 12,
							width: 299,
							height: 250,
							children: [
								// First child = branch start label node
								{ id: 'a-branch-0-label', width: 275, height: 34, x: 12, y: 12 },
								{ id: 'b', width: 275, height: 34, x: 12, y: 108 },
								{ id: 'c', width: 275, height: 34, x: 12, y: 204 }
							],
							edges: [
								{
									id: 'e-label0-b',
									sources: ['a-branch-0-label'],
									targets: ['b']
								},
								{ id: 'e-b-c', sources: ['b'], targets: ['c'] }
							]
						},
						// Lane 1
						{
							id: 'a-branch-1',
							x: 361,
							y: 12,
							width: 299,
							height: 154,
							children: [
								{ id: 'a-branch-1-label', width: 275, height: 34, x: 12, y: 12 },
								{ id: 'd', width: 275, height: 34, x: 12, y: 108 }
							],
							edges: [
								{
									id: 'e-label1-d',
									sources: ['a-branch-1-label'],
									targets: ['d']
								}
							]
						},
						// Lane 2
						{
							id: 'a-branch-2',
							x: 710,
							y: 12,
							width: 299,
							height: 346,
							children: [
								{ id: 'a-branch-2-label', width: 275, height: 34, x: 12, y: 12 },
								{ id: 'e', width: 275, height: 34, x: 12, y: 108 },
								{ id: 'f', width: 275, height: 34, x: 12, y: 204 },
								{ id: 'g', width: 275, height: 34, x: 12, y: 300 }
							],
							edges: [
								{
									id: 'e-label2-e',
									sources: ['a-branch-2-label'],
									targets: ['e']
								},
								{ id: 'e-e-f', sources: ['e'], targets: ['f'] },
								{ id: 'e-f-g', sources: ['f'], targets: ['g'] }
							]
						}
					],
					edges: []
				},
				// a__group bottom = 300+370 = 670, +62 gap = 732
				{ id: 'a-end', width: 275, height: 34, x: 385, y: 732 },
				{ id: 'Result', width: 275, height: 34, x: 385, y: 828 }
			],
			edges: [
				{ id: 'e-Trigger-Input', sources: ['Trigger'], targets: ['Input'] },
				{ id: 'e-Input-a', sources: ['Input'], targets: ['a'] },
				{ id: 'e-a-group', sources: ['a'], targets: ['a__group'] },
				{ id: 'e-group-end', sources: ['a__group'], targets: ['a-end'] },
				{ id: 'e-a-end-Result', sources: ['a-end'], targets: ['Result'] }
			],
			width: 1045,
			height: 874
		},
		null,
		2
	)
</script>
