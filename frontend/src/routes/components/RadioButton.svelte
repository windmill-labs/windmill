<script lang="ts">
	export let label = '';
	export let options: [string, any][];
	export let value: any;
	export let small = false;

	import { createEventDispatcher } from 'svelte';

	const dispatch = createEventDispatcher();
</script>

<fieldset class="mt-2 mr-4">
	<legend class="sr-only">{label}</legend>
	<div class="flex flex-row gap-2">
		{#each options as [label, val]}
			<label class:selected={val == value} class={small ? 'py-0' : 'py-3'}>
				<input
					type="radio"
					value={val}
					class="sr-only"
					bind:group={value}
					aria-labelledby="memory-option-0-label"
					on:click={() => dispatch('change', val)}
				/>
				<p>{label}</p>
			</label>
		{/each}
	</div>
</fieldset>

<style>
	label {
		@apply border rounded-md px-3 flex items-center justify-center text-sm font-medium uppercase sm:flex-1 cursor-pointer focus:outline-none;
	}

	label.selected {
		@apply border bg-blue-500/90 text-white rounded-md px-3 flex items-center justify-center text-sm font-medium uppercase sm:flex-1 cursor-pointer focus:outline-none;
	}
</style>
