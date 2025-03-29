<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher<{
		acceptAll: void
		rejectAll: void
		quitDiffMode: void
		seeHistory: void
	}>()

	export let isDiffMode = false

</script>

<div class="absolute flex flex-row bottom-10 left-1/2 z-10 -translate-x-1/2 bg-surface rounded-md">
	{#if !isDiffMode}
		<button
			class="px-2 py-1 bg-[#a0e6a0] rounded-l text-sm font-semibold text-black"
			on:click={() => dispatch('acceptAll')}>Accept all</button
		>
		<button
			class="px-2 py-1 bg-[#e6a0a0] rounded-r text-sm font-semibold text-black"
			on:click={() => dispatch('rejectAll')}>Reject all</button
		>
	{:else}
		<button
			class="px-2 py-1 bg-[#a0e6a0] rounded-l text-sm font-semibold text-black"
			on:click={() => dispatch('quitDiffMode')}>Quit diff mode</button
		>
		<button
			class="px-2 py-1 bg-[#e6a0a0] rounded-r text-sm font-semibold text-black"
			on:click={() => dispatch('seeHistory')}>See changes history</button
		>
	{/if}
</div>
