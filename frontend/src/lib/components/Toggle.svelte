<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let options: {
		left?: string
		right?: string
	} = {}
	export let checked: boolean = false

	const id = (Math.random() + 1).toString(36).substring(10)
	const dispatch = createEventDispatcher()
</script>

<span>
	<label for={id} class="inline-flex items-center cursor-pointer mt-2">
		{#if Boolean(options?.left)}
			<span class="mr-2 text-sm font-medium text-gray-900">{options?.left}</span>
		{/if}
		<div class="relative" on:click|stopPropagation={() => {}}>
			<input
				type="checkbox"
				value={false}
				{id}
				class="sr-only peer"
				bind:checked
				on:change|stopPropagation={(e) => {
					dispatch('change', checked)
				}}
			/>
			<div
				class="w-11 h-6 bg-gray-200 rounded-full peer peer-focus:ring-4 peer-focus:ring-blue-300 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"
			/>
		</div>
		{#if Boolean(options?.right)}
			<span class="ml-2 text-sm font-medium text-gray-900">{options?.right}</span>
		{/if}
	</label>
</span>
