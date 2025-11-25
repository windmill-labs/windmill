<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import ContextMenu, { type ContextMenuItem } from '../../../common/contextmenu/ContextMenu.svelte'

	interface Props {
		enableSourceHandle?: boolean
		enableTargetHandle?: boolean
		offset?: number
		wrapperClass?: string
		contextMenuItems?: ContextMenuItem[]
		children?: import('svelte').Snippet<[any]>
	}

	let {
		enableSourceHandle = true,
		enableTargetHandle = true,
		offset = 0,
		wrapperClass = '',
		contextMenuItems = undefined,
		children
	}: Props = $props()

	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

{#if contextMenuItems && contextMenuItems.length > 0}
	<ContextMenu items={contextMenuItems}>
		<div class={twMerge('relative rounded-md', wrapperClass)} style={`margin-left: ${offset}px;`}>
			{@render children?.({ darkMode })}
		</div>

		{@render handles()}
	</ContextMenu>
{:else}
	<div class={twMerge('relative rounded-md', wrapperClass)} style={`margin-left: ${offset}px;`}>
		{@render children?.({ darkMode })}
	</div>

	{@render handles()}
{/if}

{#snippet handles()}
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
{/snippet}
