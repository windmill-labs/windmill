<script lang="ts">
	import type { DropdownItem } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'
	import { faEllipsisH } from '@fortawesome/free-solid-svg-icons'
	import { Button, Menu } from './common'

	type Alignment = 'start' | 'end'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let dropdownItems: DropdownItem[]
	export let name: string | undefined = undefined
	export let placement: Placement = 'bottom-start'

	const dispatch = createEventDispatcher()
</script>

<Menu {placement} let:close>
	<Button color="light" size="xs" btnClasses="!text-blue-500 bg-transparent" slot="trigger">
		{#if !$$slots.default}
			<Icon data={faEllipsisH} scale={1.2} />
		{:else}
			<slot />
		{/if}
	</Button>
	{#if dropdownItems}
		{#each dropdownItems as item, i}
			{#if item.action}
				<button
					on:click={(event) => {
						if (!item.disabled) {
							event.preventDefault()
							close()
							item.action && item.action(event)
							dispatch('click', { item: item?.eventName })
						}
					}}
					class="block w-full whitespace-nowrap hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30 px-4 py-2 text-sm text-gray-700 text-left {item.separatorTop
						? 'border-t'
						: ''} {item.separatorBottom ? 'border-b' : ''} {item.type == 'delete'
						? 'text-red-500'
						: ''}"
					role="menuitem"
					tabindex="-1"
					id="user-menu-item-{name}-{i}}"
					disabled={item.disabled}
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
				</button>
			{:else if item.href}
				<a
					href={item.href}
					on:click={() => {
						if (!item.disabled) {
							close()
						}
					}}
					class="block w-full px-4 py-2 text-sm text-gray-700 hover:drop-shadow-sm hover:bg-gray-50 hover:bg-opacity-30"
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
					class="block  px-4 py-2 text-sm text-gray-700"
					role="menuitem"
					tabindex="-1"
					id="user-menu-item-{name}-{i}}"
				>
					{item.displayName}
				</span>
			{/if}
		{/each}
	{/if}
</Menu>
