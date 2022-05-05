<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let tabs: [string, string][];
	export let dflt = 0;
	export let tab = tabs[dflt][0];

	const dispatch = createEventDispatcher();
	$: dispatch('update', tab);
</script>

<div class="flex flex-col sm:flex-row text-base">
	{#each tabs as [t, label]}
		<button
			class=" py-1 px-6 block hover:text-blue-500 focus:outline-noneborder-gray-200  
				{t === tab ? 'text-gray-700 font-semibold underline' : 'text-gray-500'}"
			on:click={() => {
				tab = t;
			}}
		>
			{label}
		</button>
	{/each}
</div>
