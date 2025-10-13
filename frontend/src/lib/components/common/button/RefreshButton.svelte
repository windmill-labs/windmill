<script lang="ts">
	import { Button, ButtonType } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		loading: boolean
		size?: ButtonType.Size
		light?: boolean
	}

	let { loading, size = 'xs2', light = false }: Props = $props()

	let buttonHover = $state(false)
</script>

<Popover>
	<Button
		on:mouseenter={() => (buttonHover = true)}
		on:mouseleave={() => (buttonHover = false)}
		color="light"
		{size}
		variant="border"
		on:click
	>
		<RefreshCw
			class={twMerge(loading ? 'animate-spin ' : '', light ? 'text-secondary' : '')}
			size="14"
		/>
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
