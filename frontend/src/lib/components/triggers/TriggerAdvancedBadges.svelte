<script lang="ts">
	import Badge from '$lib/components/Badge.svelte'
	import type { Retry } from '$lib/gen'

	interface Props {
		error_handler_path?: string | undefined
		retry?: Retry | undefined
		extraBadges?: { name: string; active: boolean }[]
	}

	let { error_handler_path = undefined, retry = undefined, extraBadges = [] }: Props = $props()

	let allBadges = $derived(
		[
			{ name: 'Error Handler', active: !!error_handler_path },
			{ name: 'Retries', active: !!retry },
			...extraBadges
		].filter((b) => b.active)
	)
</script>

{#if allBadges.length > 0}
	<div class="flex grow min-w-0 w-full flex-wrap gap-1 ps-2">
		{#each allBadges as badge}
			<Badge twBgColor="bg-surface-sunken" twTextColor="text-primary">{badge.name}</Badge>
		{/each}
	</div>
{/if}
