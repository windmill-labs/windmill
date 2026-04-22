<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte'
	import { Badge } from '$lib/components/common'
	import { NODE } from '../../util'
	import type { WacDagNode } from '$lib/infer'

	let { data }: { data: { dagNode: WacDagNode } } = $props()

	let dagNode = $derived(data.dagNode)
	let isInline = $derived(dagNode.node_type.type === 'InlineStep')
	let label = $derived(dagNode.label)
	let script = $derived(dagNode.node_type.type === 'Step' ? dagNode.node_type.script : undefined)
	let hasExternalPath = $derived(script !== undefined && script !== label)
</script>

<div class="relative">
	<div
		class="w-full flex relative rounded-md drop-shadow-sm bg-surface-tertiary overflow-hidden"
		style="width: {NODE.width}px; height: {NODE.height}px;"
	>
		<div class="flex flex-row items-center w-full p-2 text-2xs text-primary rounded-md gap-2">
			<div class="flex flex-col flex-grow min-w-0">
				<div class="truncate text-center text-emphasis">{label}</div>
			</div>
			{#if hasExternalPath}
				<Badge color="blue" baseClass="max-w-[100px]" title={script}>
					<span class="text-2xs truncate">{script}</span>
				</Badge>
			{:else if isInline}
				<Badge color="indigo" baseClass="max-w-[60px]">
					<span class="text-2xs">inline</span>
				</Badge>
			{/if}
		</div>
	</div>
</div>

<Handle type="target" position={Position.Top} isConnectable={false} />
<Handle type="source" position={Position.Bottom} isConnectable={false} />
