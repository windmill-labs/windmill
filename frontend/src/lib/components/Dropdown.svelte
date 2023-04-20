<script lang="ts">
	import type { DropdownItem } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { MoreVertical } from 'lucide-svelte'
	import { Button, Menu } from './common'
	import { goto } from '$app/navigation'
	import { twMerge } from 'tailwind-merge'

	type Alignment = 'start' | 'end'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let dropdownItems: DropdownItem[] | (() => DropdownItem[]) = []
	export let name: string | undefined = undefined
	export let placement: Placement = 'bottom-start'
	export let btnClasses = ''
	$: buttonClass = twMerge('!border-0 bg-transparent !p-[6px]', btnClasses)

	const dispatch = createEventDispatcher()

	function computeDropdowns(): DropdownItem[] {
		if (typeof dropdownItems === 'function') {
			return dropdownItems()
		} else {
			return dropdownItems
		}
	}
</script>

<Menu {placement} let:close>
	<Button
		nonCaptureEvent
		color="dark"
		variant="border"
		size="xs"
		btnClasses={buttonClass}
		{...$$restProps}
		slot="trigger"
	>
		{#if !$$slots.default}
			<MoreVertical size={20} />
		{:else}
			<slot />
		{/if}
	</Button>
	{#if dropdownItems}
		{#each computeDropdowns() as item, i}
			{#if item.action}
				<button
					on:click|preventDefault|stopPropagation={(event) => {
						if (!item.disabled) {
							close()
							item.action && item.action(event)
							dispatch('click', { item: item?.eventName })
						}
					}}
					class="block w-full whitespace-nowrap hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30
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
					on:click|stopPropagation|preventDefault={() => goto(item.href ?? '')}
					class="block w-full px-4 font-semibold text-left py-2 text-sm text-gray-700 hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30
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
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<span
					class:bg-gray-200={item.disabled}
					class="block font-semibold text-left px-4 py-2 text-sm text-gray-700 cursor-auto"
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
