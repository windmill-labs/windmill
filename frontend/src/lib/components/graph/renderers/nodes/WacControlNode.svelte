<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { NODE } from '../../util'
	import type { WacDagNode } from '$lib/infer'

	let { data }: { data: { dagNode: WacDagNode } } = $props()

	let dagNode = $derived(data.dagNode)
	let nodeType = $derived(dagNode.node_type.type)

	let displayLabel = $derived.by(() => {
		const nt = dagNode.node_type
		switch (nt.type) {
			case 'Branch':
				return nt.condition_source
			case 'ParallelStart':
				return 'parallel'
			case 'ParallelEnd':
				return 'join'
			case 'LoopStart':
				return `${dagNode.label} (${nt.iter_source})`
			case 'LoopEnd':
				return dagNode.label
			case 'Sleep':
				return `sleep(${nt.seconds}s)`
			case 'WaitForApproval':
				return 'wait for approval'
			case 'Return':
				return 'return'
			default:
				return dagNode.label
		}
	})

	let bgClass = $derived.by(() => {
		switch (nodeType) {
			case 'Return':
				return 'bg-surface-tertiary'
			default:
				return 'bg-component-virtual-node'
		}
	})
</script>

<div class="relative">
	<div
		class="w-full flex relative rounded-md drop-shadow-sm {bgClass} overflow-hidden"
		style="width: {NODE.width}px; height: {NODE.height}px;"
	>
		<div class="flex flex-row items-center justify-center w-full p-2 text-2xs rounded-md">
			<div class="truncate text-center text-emphasis">{displayLabel}</div>
		</div>
	</div>
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
