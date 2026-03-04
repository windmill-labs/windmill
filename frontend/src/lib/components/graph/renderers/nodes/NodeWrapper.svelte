<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import ContextMenu, { type ContextMenuItem } from '../../../common/contextmenu/ContextMenu.svelte'
	import { getGraphContext } from '../../graphContext'

	interface Props {
		enableSourceHandle?: boolean
		enableTargetHandle?: boolean
		wrapperClass?: string
		contextMenuItems?: ContextMenuItem[]
		/** xyflow node ID — used to fade nodes that are part of a moving subflow */
		nodeId?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		enableSourceHandle = true,
		enableTargetHandle = true,
		wrapperClass = '',
		contextMenuItems = undefined,
		nodeId = undefined,
		children
	}: Props = $props()

	const { moveManager } = getGraphContext()

	let faded = $derived(
		nodeId != null && (moveManager?.draggedNodeIds?.has(nodeId) ?? false)
	)

	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

{#if contextMenuItems && contextMenuItems.length > 0}
	<ContextMenu items={contextMenuItems}>
		<div class={twMerge('relative rounded-md', faded ? 'opacity-30' : '', wrapperClass)}>
			{@render children?.({ darkMode })}
		</div>

		{@render handles()}
	</ContextMenu>
{:else}
	<div class={twMerge('relative rounded-md', faded ? 'opacity-30' : '', wrapperClass)}>
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
		/>
	{/if}

	{#if enableTargetHandle}
		<Handle
			type="target"
			isConnectable={false}
			position={Position.Top}
		/>
	{/if}
{/snippet}
