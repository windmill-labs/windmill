<script lang="ts">
	import { createContextMenu, melt } from '@melt-ui/svelte'
	import { fly } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'

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
	}

	let { items = [], children, menu, class: className = '', onItemClick }: Props = $props()

	const {
		elements: { menu: menuElement, item, trigger },
		states: { open }
	} = createContextMenu({
		positioning: {
			placement: 'right-start'
		},
		preventScroll: true,
		closeOnOutsideClick: true
	})

	function handleItemClick(menuItem: ContextMenuItem) {
		if (!menuItem.disabled) {
			menuItem.onClick?.()
			onItemClick?.(menuItem)
		}
	}

	function handleContextMenu(event: MouseEvent) {
		console.log('Context menu event detected:', event)
		// Let Melt UI handle this
	}

	$inspect('dbg context menu', $open)
</script>

<div
	use:melt={$trigger}
	class={twMerge('block w-full h-full', className)}
	role="button"
	tabindex="0"
	aria-label="Right click for context menu"
	oncontextmenu={handleContextMenu}
	data-context-menu-trigger
>
	{@render children?.()}
</div>

{#if $open}
	<div
		class="z-50 flex flex-col gap-1 min-w-[12rem] overflow-hidden rounded-md border bg-surface p-1 shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2"
		use:melt={$menuElement}
		transition:fly={{ duration: 150, y: -10 }}
	>
		{#each items as menuItem (menuItem.id)}
			{#if menuItem.divider}
				<div class="my-1 h-px bg-border-light"></div>
			{:else}
				<div
					class={twMerge(
						'relative flex cursor-default select-none items-center rounded-md px-2 py-1.5 text-xs outline-none transition-colors',
						menuItem.disabled
							? 'pointer-events-none opacity-50'
							: 'data-[highlighted]:bg-surface-hover'
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
