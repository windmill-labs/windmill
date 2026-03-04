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
		wrapperClass = '',
		contextMenuItems = undefined,
		menuItems = undefined,
		nodeId = undefined,
		children
	}: Props = $props()

	let resolvedContextMenuItems: ContextMenuItem[] | undefined = $derived(
		contextMenuItems ??
			menuItems?.flatMap((item) => [
				...(item.separatorTop ? [{ id: `${item.displayName}-divider`, label: '', divider: true }] : []),
				{
					id: item.displayName,
					label: item.displayName,
					icon: item.icon,
					disabled: item.disabled,
					type: item.type,
					shortcut: item.shortcut,
					onClick: item.action as (() => void) | undefined
				}
			])
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
