<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'

	let {
		title = '',
		tooltip = undefined,
		items = $bindable([] as string[]),
		placeholder = 'Add filter (e.g. f/**)',
		emptyHint = 'No filter yet'
	} = $props()

	let newItem = $state('')
	let inputRef: TextInput | undefined = $state(undefined)

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

<div class="flex flex-col gap-1 h-full flex-1">
	<div class="flex items-center gap-2 mb-1">
		<h4 class="font-semibold text-sm">{title}</h4>
		{#if tooltip}
			{@render tooltip?.()}
		{/if}
	</div>

	<div
		class="flex-1 min-h-20 overflow-y-auto rounded-md border border-border-light p-2 flex flex-wrap gap-2 {items.length ===
		0
			? 'items-center justify-center'
			: 'content-start'}"
	>
		{#if items.length === 0}
			<div class="text-center text-xs text-hint">{emptyHint}</div>
		{:else}
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
		{/if}
	</div>

	<!-- Its own row at the bottom, so adding or removing filters above does not move it. -->
	<div class="flex items-center gap-1 mt-auto pt-1">
		<TextInput
			bind:this={inputRef}
			bind:value={newItem}
			size="sm"
			class="flex-1 min-w-0"
			inputProps={{
				placeholder,
				onkeydown: (e: KeyboardEvent) => {
					if (e.key === 'Enter') {
						e.preventDefault()
						addItem()
					}
				}
			}}
		/>
		<Button
			variant="default"
			unifiedSize="sm"
			iconOnly
			startIcon={{ icon: Plus }}
			title="Add filter"
			onClick={addItem}
		/>
	</div>
</div>
