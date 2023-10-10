<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let type: 'text' | 'badge' | 'checkbox' = 'text'
	export let value: any
</script>

{#if type === 'badge'}
	<Badge>
		{value}
	</Badge>
{:else}
	{value}
{/if}
