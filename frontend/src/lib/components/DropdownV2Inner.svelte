<script lang="ts">
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	type Item = {
		displayName: string
		action?: (e: CustomEvent<any>) => void
		icon?: any
		href?: string
		disabled?: boolean
		type?: 'action' | 'delete'
		hide?: boolean | undefined
	}

	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []

	let computedItems: Item[] | undefined = undefined
	async function computeItems() {
		if (typeof items === 'function') {
			computedItems = ((await items()) ?? []).filter((item) => !item.hide)
		} else {
			computedItems = items.filter((item) => !item.hide)
		}
	}

	computeItems()
</script>

{#if computedItems}
	<div class="flex flex-col">
		{#each computedItems ?? [] as item}
			<MenuItem
				on:click={(e) => item?.action?.(e)}
				href={item?.href}
				disabled={item?.disabled}
				class={twMerge(
					'px-4 py-2 text-primary hover:bg-surface-hover hover:text-primary cursor-pointer text-xs transition-all',
					'flex flex-row gap-2 items-center',
					item?.disabled && 'text-gray-400 cursor-not-allowed',
					item?.type === 'delete' &&
						!item?.disabled &&
						'text-red-500 hover:bg-red-100 hover:text-red-500'
				)}
			>
				{#if item.icon}
					<svelte:component this={item.icon} size={14} />
				{/if}
				{item.displayName}
			</MenuItem>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin mx-auto p-4" size={24} />
{/if}
