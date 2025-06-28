<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		enableSourceHandle?: boolean
		enableTargetHandle?: boolean
		offset?: number
		wrapperClass?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		enableSourceHandle = true,
		enableTargetHandle = true,
		offset = 0,
		wrapperClass = '',
		children
	}: Props = $props()

	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

<div
	class={twMerge('relative shadow-md rounded-sm', wrapperClass)}
	style={`margin-left: ${offset}px;`}
>
	{@render children?.({ darkMode })}
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
