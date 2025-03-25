<script lang="ts">
	import { Button } from '$lib/components/common'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { createEventDispatcher } from 'svelte'
	import { CornerDownLeft, Play } from 'lucide-svelte'
	import type { Item } from '$lib/utils'

	const dispatch = createEventDispatcher()

	export let testIsLoading = false
	export let cancelLoading = false
	export let dropdownItems: Item[] = []

	$: if (cancelLoading && !testIsLoading) {
		cancelLoading = false
	}

	function handleClick() {
		if (testIsLoading) {
			dispatch('cancel')
		} else {
			dispatch('run')
		}
	}
</script>

<Button
	color={testIsLoading ? 'red' : 'dark'}
	btnClasses="w-28"
	size="xs"
	on:click={handleClick}
	shortCut={testIsLoading
		? undefined
		: {
				Icon: CornerDownLeft
		  }}
	{dropdownItems}
	dropdownDisabled={testIsLoading}
>
	<div class="flex flex-row items-center gap-2 align-left">
		{#if testIsLoading}
			<WindmillIcon white={true} class="text-white" height="16px" width="16px" spin="fast" />
			{cancelLoading ? 'Canceling...' : 'Cancel'}
		{:else}
			<Play size={16} />
			Run
		{/if}
	</div>
	<svelte:fragment slot="dropdownIcon">
		<slot name="dropdownIcon" />
	</svelte:fragment>
</Button>
