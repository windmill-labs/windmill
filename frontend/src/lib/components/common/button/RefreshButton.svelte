<script lang="ts">
	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	import Popover from '$lib/components/Popover.svelte'

	interface Props {
		loading: boolean
	}

	let { loading }: Props = $props()

	let buttonHover = $state(false)
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
	{#snippet text()}
		{#if loading}
			{#if buttonHover}
				Stop Refreshing
			{:else}
				Refreshing...
			{/if}
		{:else}
			Refresh
		{/if}
	{/snippet}
</Popover>
