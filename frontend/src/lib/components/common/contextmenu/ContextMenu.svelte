<script lang="ts">
	import { createContextMenu, melt } from '@melt-ui/svelte'
	import { fly } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'
	import { pointerDownOutside } from '$lib/utils'
	import {
		getContextMenuContainerClass,
		CONTEXT_MENU_ITEM_BASE_CLASS,
		CONTEXT_MENU_ITEM_HOVER_MELT_CLASS,
		CONTEXT_MENU_ITEM_DISABLED_CLASS,
		CONTEXT_MENU_DIVIDER_CLASS,
		CONTEXT_MENU_ANIMATION_CLASSES
	} from './contextMenuStyles'

	export interface ContextMenuItem {
		id: string
		label: string
		icon?: any
		disabled?: boolean
		onClick?: () => void
		divider?: boolean
	}

	interface Props {
		items?: ContextMenuItem[]
		children?: Snippet
		menu?: Snippet<[{ item: ContextMenuItem }]>
		class?: string
		onItemClick?: (item: ContextMenuItem) => void
		closeOnOutsideClick?: boolean
	}

	let {
		items = [],
		children,
		menu,
		class: className = '',
		onItemClick,
		closeOnOutsideClick = true
	}: Props = $props()

	const {
		elements: { menu: menuElement, item, trigger },
		states: { open }
	} = createContextMenu({
		positioning: {
			placement: 'right-start'
		},
		preventScroll: true,
		closeOnOutsideClick: false
	})

	function handleItemClick(menuItem: ContextMenuItem) {
		if (!menuItem.disabled) {
			menuItem.onClick?.()
			onItemClick?.(menuItem)
		}
	}

	function close() {
		open.set(false)
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-context-menu]')) as HTMLElement[]
	}

	function handlePointerDownOutside() {
		if (closeOnOutsideClick && open.get()) {
			close()
		}
	}
</script>

<div
	use:melt={$trigger}
	class={twMerge('block w-full h-full', className)}
	role="button"
	tabindex="0"
	aria-label="Right click for context menu"
	use:pointerDownOutside={{
		capture: true,
		stopPropagation: false,
		exclude: getMenuElements,
		onClickOutside: handlePointerDownOutside
	}}
	data-context-menu-trigger
>
	{@render children?.()}
</div>

{#if $open}
	<div
		class="{getContextMenuContainerClass()} {CONTEXT_MENU_ANIMATION_CLASSES}"
		use:melt={$menuElement}
		transition:fly={{ duration: 150, y: -10 }}
		data-context-menu
	>
		{#each items as menuItem (menuItem.id)}
			{#if menuItem.divider}
				<div class={CONTEXT_MENU_DIVIDER_CLASS}></div>
			{:else}
				<div
					class={twMerge(
						CONTEXT_MENU_ITEM_BASE_CLASS,
						menuItem.disabled
							? CONTEXT_MENU_ITEM_DISABLED_CLASS
							: CONTEXT_MENU_ITEM_HOVER_MELT_CLASS
					)}
					use:melt={$item}
					onclick={() => handleItemClick(menuItem)}
				>
					{#if menuItem.icon}
						<menuItem.icon size={14} class="mr-2" />
					{/if}
					{#if menu}
						{@render menu({ item: menuItem })}
					{:else}
						<span>{menuItem.label}</span>
					{/if}
				</div>
			{/if}
		{/each}
	</div>
{/if}
