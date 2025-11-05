<script lang="ts">
	import { Button, ButtonType } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	import Popover from '$lib/components/Popover.svelte'

	interface Props {
		loading: boolean
		size?: ButtonType.UnifiedSize
		light?: boolean
		onClick?: () => void
	}

	let { loading, size = 'md', onClick }: Props = $props()

	let buttonHover = $state(false)
</script>

<Popover>
	<Button
		on:mouseenter={() => (buttonHover = true)}
		on:mouseleave={() => (buttonHover = false)}
		color="light"
		unifiedSize={size}
		variant="subtle"
		{onClick}
		startIcon={{ icon: RefreshCw, props: { class: loading ? 'animate-spin' : '' } }}
	></Button>
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
