<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	export let value: any
	export let options: {
		left: { label?: string; value: any }
		right: { label?: string; value: any }
	}
	let checked = options.right.value === value

	const id = Date.now().toString(36)

	const dispatch = createEventDispatcher()
</script>

<span>
	<label for={id} class="inline-flex items-center cursor-pointer mt-2">
		{#if Boolean(options.left.label)}
			<span class="mr-2 text-sm font-medium text-gray-900">{options.left.label}</span>
		{/if}
		<div class="relative">
			<input
				type="checkbox"
				value={false}
				{id}
				class="sr-only peer"
				bind:checked
				on:change={() => {
					value = checked ? options.right.value : options.left.value
					dispatch('change', value)
				}}
			/>
			<div
				class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"
			/>
		</div>
		{#if Boolean(options.right.label)}
			<span class="ml-2 text-sm font-medium text-gray-900">{options.right.label}</span>
		{/if}
	</label>
</span>
