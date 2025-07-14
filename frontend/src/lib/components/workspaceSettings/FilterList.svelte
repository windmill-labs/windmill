<script lang="ts">
	import { Plus, X } from 'lucide-svelte'

	let {
		title = '',
		tooltip = undefined,
		items = $bindable([] as string[]),
		placeholder = 'Add filter (e.g. f/**)'
	} = $props()

	let newItem = $state('')
	let inputRef: HTMLInputElement | null = $state(null)

	function addItem() {
		const value = newItem.trim()
		if (value && !items.includes(value)) {
			items = [...items, value]
			newItem = ''
			inputRef?.focus()
		}
	}

	function removeItem(idx: number) {
		items = items.filter((_, i) => i !== idx)
	}
</script>

<div class="flex flex-col gap-1">
	<div class="flex items-center gap-2 mb-1">
		<h4 class="font-semibold text-sm">{title}</h4>
		{#if tooltip}
			{@render tooltip?.()}
		{/if}
	</div>

	<div class="flex flex-wrap gap-2 items-center mb-1">
		{#each items as item, idx (item)}
			<span class="flex items-center bg-gray-100 rounded-full px-3 py-1 text-xs text-gray-700">
				{item}
				<button
					class="ml-2 text-gray-400 hover:text-red-500 focus:outline-none"
					onclick={() => removeItem(idx)}
					aria-label="Remove filter"
				>
					<X size={14} />
				</button>
			</span>
		{/each}

		<input
			bind:this={inputRef}
			class="border border-gray-300 rounded-full px-3 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-primary"
			{placeholder}
			value={newItem}
			oninput={(e) => (newItem = e.currentTarget.value)}
			onkeydown={(e) => e.key === 'Enter' && (addItem(), e.preventDefault())}
		/>
		<button
			class="ml-1 text-primary hover:bg-primary/10 rounded-full p-1"
			onclick={addItem}
			aria-label="Add filter"
		>
			<Plus size={14} />
		</button>
	</div>
</div>
