<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'

	export let enableSourceHandle: boolean = true
	export let enableTargetHandle: boolean = true
	export let offset: number = 0
	export let wrapperClass: string = ''

	let darkMode: boolean = false
</script>

<DarkModeObserver bind:darkMode />

<div
	class={twMerge('relative shadow-md rounded-sm', wrapperClass)}
	style={`margin-left: ${offset}px;`}
>
	<slot {darkMode} />
</div>

{#if enableSourceHandle}
	<Handle
		type="source"
		isConnectable={false}
		position={Position.Bottom}
		style={`margin-left: ${offset / 2}px;`}
	/>
{/if}

{#if enableTargetHandle}
	<Handle
		type="target"
		isConnectable={false}
		position={Position.Top}
		style={`margin-left: ${offset / 2}px;`}
	/>
{/if}
