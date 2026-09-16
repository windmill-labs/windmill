<script lang="ts">
	import { untrack } from 'svelte'
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuBuilders, MenubarMenuElements } from '@melt-ui/svelte'
	import { Building, ChevronRight } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { sidebarClasses } from './MenuButton.svelte'
	import WorkspacePickerBody from './WorkspacePickerBody.svelte'
	import { MenuItem } from '$lib/components/meltComponents'
	import { clearWorkspaceFromStorage } from '$lib/stores'
	import { base } from '$lib/base'

	interface Props {
		// Submenu builders from the enclosing melt Menu — createSubmenu must be
		// called against this specific menu instance (same pattern as DropdownSubmenuItem).
		builders: MenubarMenuBuilders
		// Melt uses the parent menu's item builder for submenu rows too, so the
		// picker body's MenuItems work here unchanged.
		item: MenubarMenuElements['item']
	}

	let { builders, item }: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = untrack(() => builders).createSubmenu()
</script>

<button
	use:melt={$subTrigger}
	class={twMerge(
		'flex flex-row gap-3.5 items-center px-2 py-2 w-full',
		sidebarClasses.text,
		'transition-colors',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)}
>
	<Building size={14} class="shrink-0" />
	<span class="grow text-left">Switch workspace</span>
	<ChevronRight size={14} class="shrink-0 text-tertiary" />
</button>

{#if $subOpen}
	<div
		use:melt={$subMenu}
		class="z-[6001] w-56 bg-surface border rounded-md shadow-md focus:outline-none"
	>
		<WorkspacePickerBody {item} strictWorkspaceSelect />
		<!-- The picker's list scrolls inside itself, so this sits below it and stays reachable
		     however many workspaces the operator belongs to. -->
		<div class="border-t border-border-light" role="none">
			<MenuItem
				href="{base}/user/workspaces"
				onClick={() => clearWorkspaceFromStorage()}
				class={twMerge(
					'w-full flex flex-row gap-2 px-4 py-2 font-normal',
					sidebarClasses.text,
					'hover:bg-surface-hover hover:text-primary',
					'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
				)}
				{item}
			>
				All workspaces
			</MenuItem>
		</div>
	</div>
{/if}
