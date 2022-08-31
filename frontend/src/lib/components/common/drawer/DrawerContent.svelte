<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let title: string | undefined = undefined
	const dispatch = createEventDispatcher()
</script>

<div class="divide-y h-screen">
	<div class="flex justify-between p-2">
		{title}
		<button on:click={() => dispatch('close')}>Close</button>
	</div>

	<div class="p-2">
		<slot />
	</div>
</div>
