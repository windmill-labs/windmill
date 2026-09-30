<!--
@component
A pipeline node's actions, reachable two ways: the kebab revealed on hover at the
node's top-right corner, and a right-click menu over the node carrying the same
items. Place it inside the node's `relative` wrapper, around the node's card.
-->
<script lang="ts">
	import type { Snippet } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { EllipsisVertical } from 'lucide-svelte'
	import { stopPropagation, preventDefault } from 'svelte/legacy'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ContextMenu from '$lib/components/common/contextmenu/ContextMenu.svelte'
	import { contextMenuItemsFromMenu } from '$lib/components/common/contextmenu/fromMenuItems'
	import type { Item } from '$lib/utils'

	interface Props {
		items: Item[]
		/** The pointer is over the node: shows the kebab. */
		hover: boolean
		children: Snippet
	}

	let { items, hover, children }: Props = $props()

	let menuOpen = $state(false)
	let contextItems = $derived(contextMenuItemsFromMenu(items))
</script>

{#if items.length > 0}
	<ContextMenu items={contextItems}>
		{@render children()}
	</ContextMenu>
	<!-- Rendered only on hover or while open, so the canvas stays clean at rest.
	     `pointerdown` is stopped so svelte-flow doesn't select the node when the
	     user reaches for the menu. -->
	<div class="absolute -top-2 -right-2 h-7 p-1 min-w-7" style="will-change: transform;">
		<DropdownV2
			{items}
			placement="bottom-end"
			bind:open={menuOpen}
			fixedHeight={false}
			usePointerDownOutside
		>
			{#snippet buttonReplacement()}
				<button
					class={twMerge(
						'center-center p-1 text-secondary shadow-sm bg-surface duration-0 hover:bg-surface-tertiary',
						hover || menuOpen ? 'block' : '!hidden',
						'shadow-md rounded-md'
					)}
					onpointerdown={stopPropagation(preventDefault(() => {}))}
					title="Actions"
				>
					<EllipsisVertical size={12} />
				</button>
			{/snippet}
		</DropdownV2>
	</div>
{:else}
	{@render children()}
{/if}
