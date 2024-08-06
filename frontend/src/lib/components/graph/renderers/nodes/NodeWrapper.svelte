<script lang="ts">
	// @ts-ignore
	import { Handle, NodeToolbar, Position, type NodeProps } from '@xyflow/svelte'

	export let enableSourceHandle: boolean = true
	export let enableTargetHandle: boolean = true
	export let offset: number = 0
</script>

<div class="relative" style={`margin-left: ${offset}px;`}>
	<div class="absolute -top-6 z-50">
		<slot name="header" />
	</div>

	<slot />
</div>

{#if enableSourceHandle}
	<Handle type="source" position={Position.Bottom} />
{/if}

{#if enableTargetHandle}
	<Handle type="target" position={Position.Top} />
{/if}
