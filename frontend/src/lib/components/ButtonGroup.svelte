<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let value: any
	export let options: Array<{ label: string; value: any }>

	const dispatch = createEventDispatcher()

	$: active = options.findIndex((option) => option.value === value)
</script>

<div class="inline-flex rounded-md shadow-sm" role="group">
	{#each options as button, index}
		<button
			type="button"
			on:click={() => {
				value = button.value
				dispatch('change', value)
			}}
			class={`
      ${
				index === 0
					? 'rounded-l-md border-t border-b border-l'
					: index === options.length - 1
					? 'rounded-r-md border-t border-b border-r'
					: 'border'
			}
      ${
				active === index ? 'text-blue-700' : 'text-gray-900'
			} py-2 px-4 text-sm font-medium bg-white border-gray-200 hover:bg-gray-100 hover:text-blue-700 focus:z-10 focus:ring-2 focus:ring-blue-700 focus:text-blue-700`}
		>
			{button.label}
		</button>
	{/each}
</div>
