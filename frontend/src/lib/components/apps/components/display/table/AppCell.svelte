<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let type: 'text' | 'badge' | 'link' = 'text'
	export let value: any
</script>

{#if type === 'badge'}
	<Badge>
		{value}
	</Badge>
{:else if type === 'link'}
	<a href={value} class="underline" target="_blank">{value}</a>
{:else}
	{value}
{/if}
