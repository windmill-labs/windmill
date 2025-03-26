<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher<{
		acceptAll: void
		rejectAll: void
	}>()

	export let isDiffMode = false

</script>

<div class="absolute flex flex-row bottom-10 left-1/2 z-10 -translate-x-1/2 bg-surface rounded-md">
	{#if !isDiffMode}
		<button
			class="px-2 py-1 bg-[#a0e6a0] rounded-l text-sm font-semibold text-black"
			on:click={() => dispatch('acceptAll')}>Accept all</button
		>
	{/if}	
	<button
		class={`px-2 py-1 bg-[#e6a0a0] rounded-r text-sm font-semibold text-black ${
			isDiffMode ? 'rounded-l' : ''
		}`}
		on:click={() => dispatch('rejectAll')}>{isDiffMode ? 'Quit diff mode' : 'Reject all'}</button
	>
</div>
