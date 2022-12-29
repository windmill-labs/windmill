<script lang="ts">
	import type { DropdownItem } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { MoreHorizontal } from 'lucide-svelte'
	import { Button, Menu } from './common'

	type Alignment = 'start' | 'end'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let dropdownItems: DropdownItem[]
	export let name: string | undefined = undefined
	export let placement: Placement = 'bottom-start'
	export let btnClasses = '!text-blue-500 bg-transparent'

	const dispatch = createEventDispatcher()
</script>

<Menu {placement} let:close>
	<Button nonCaptureEvent color="light" size="xs" {btnClasses} slot="trigger">
		{#if !$$slots.default}
			<MoreHorizontal size={20} />
		{:else}
			<slot />
		{/if}
	</Button>
	{#if dropdownItems}
		{#each dropdownItems as item, i}
			{#if item.action}
				<button
					on:click|preventDefault|stopPropagation={(event) => {
						if (!item.disabled) {
							close()
							item.action && item.action(event)
							dispatch('click', { item: item?.eventName })
						}
					}}
					class="block w-full  whitespace-nowrap hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30
					 px-4 py-2 text-sm text-gray-700 text-left 
					 {item.disabled ? 'bg-gray-200' : ''}
					 {item.separatorTop ? 'border-t' : ''} {item.separatorBottom ? 'border-b' : ''} {item.type ==
					'delete'
						? 'text-red-500'
						: ''}"
					role="menuitem"
					tabindex="-1"
					id="user-menu-item-{name}-{i}}"
					disabled={item.disabled}
				>
					{#if item.icon}
						<Icon
							data={item.icon}
							scale={0.6}
							class="inline mr-2 {item.type == 'delete' ? 'text-red-500' : 'text-gray-700'}"
						/>
					{/if}
					{item.displayName}
				</button>
			{:else if item.href && !item.disabled}
				<a
					href={item.href}
					on:click|stopPropagation
					class="block w-full px-4 font-semibold text-left  py-2 text-sm text-gray-700 hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30
					{item.disabled ? 'bg-gray-200' : ''}"
					role="menuitem"
					tabindex="-1"
					id="user-menu-item-{name}-{i}}"
					class:disabled={item.disabled}
				>
					{#if item.icon}
						<Icon
							data={item.icon}
							scale={0.6}
							class="inline mr-2 {item.type == 'delete' ? 'text-red-500' : 'text-gray-700'}"
						/>
					{/if}
					{item.displayName}
				</a>
			{:else}
				<span
					class:bg-gray-50={item.disabled}
					class="block  text-left px-4 py-2 text-sm text-gray-700 cursor-auto"
					role="menuitem"
					tabindex="-1"
					id="user-menu-item-{name}-{i}}"
					on:click|preventDefault|stopPropagation
				>
					{#if item.icon}
						<Icon
							data={item.icon}
							scale={0.6}
							class="inline mr-2 {item.type == 'delete' ? 'text-red-500' : 'text-gray-700'}"
						/>
					{/if}
					{item.displayName}
				</span>
			{/if}
		{/each}
	{/if}
</Menu>
