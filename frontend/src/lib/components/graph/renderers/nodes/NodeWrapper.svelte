<script lang="ts">
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import { Handle, Position } from '@xyflow/svelte'
	import { twMerge } from 'tailwind-merge'
	import ContextMenu, { type ContextMenuItem } from '../../../common/contextmenu/ContextMenu.svelte'
	import { getGraphContext } from '../../graphContext'
	import type { Item } from '$lib/utils'

	interface Props {
		enableSourceHandle?: boolean
		enableTargetHandle?: boolean
		offset?: number
		wrapperClass?: string
		contextMenuItems?: ContextMenuItem[]
		menuItems?: Item[]
		/** xyflow node ID — used to fade nodes that are part of a moving subflow */
		nodeId?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		enableSourceHandle = true,
		enableTargetHandle = true,
		offset = 0,
		wrapperClass = '',
		contextMenuItems = undefined,
		menuItems = undefined,
		nodeId = undefined,
		children
	}: Props = $props()

	let resolvedContextMenuItems: ContextMenuItem[] | undefined = $derived(
		contextMenuItems ??
			menuItems?.map((item) => ({
				id: item.displayName,
				label: item.displayName,
				icon: item.icon,
				disabled: item.disabled,
				onClick: item.action as (() => void) | undefined
			}))
	)

	const { moveManager } = getGraphContext()

	let faded = $derived(
		nodeId != null && (moveManager?.draggedNodeIds?.has(nodeId) ?? false)
	)

	let darkMode: boolean = $state(false)
</script>

<DarkModeObserver bind:darkMode />

{#if resolvedContextMenuItems && resolvedContextMenuItems.length > 0}
	<ContextMenu items={resolvedContextMenuItems}>
		<div class={twMerge('relative rounded-md', faded ? 'opacity-30' : '', wrapperClass)} style={`margin-left: ${offset}px;`}>
			{@render children?.({ darkMode })}
		</div>

		{@render handles()}
	</ContextMenu>
{:else}
	<div class={twMerge('relative rounded-md', faded ? 'opacity-30' : '', wrapperClass)} style={`margin-left: ${offset}px;`}>
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
