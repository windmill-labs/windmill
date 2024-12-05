<script lang="ts">
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	import Popover from '$lib/components/Popover.svelte'

	export let loading: boolean

	let buttonHover = false
</script>

<Popover>
	<Button
		on:mouseenter={() => (buttonHover = true)}
		on:mouseleave={() => (buttonHover = false)}
		color="light"
		size="xs2"
		variant="border"
		on:click
	>
		<RefreshCw class={loading ? 'animate-spin ' : ''} size="14" />
	</Button>
	<svelte:fragment slot="text">
		{#if loading}
			{#if buttonHover}
				Stop Refreshing
			{:else}
				Refreshing...
			{/if}
		{:else}
			Refresh
		{/if}
	</svelte:fragment>
</Popover>
