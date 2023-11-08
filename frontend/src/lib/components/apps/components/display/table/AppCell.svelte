<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'

	export let type: 'text' | 'badge' | 'link' = 'text'
	export let value: any

	type LinkObject = {
		href: string
		label: string
	}

	function isLinkObject(value: any): value is LinkObject {
		return value && typeof value === 'object' && 'href' in value && 'label' in value
	}
</script>

{#if type === 'badge'}
	<Badge>
		{value}
	</Badge>
{:else if type === 'link'}
	{#if isLinkObject(value)}
		<a href={value.href} class="underline" target="_blank">
			{value.label}
		</a>
	{:else}
		<a href={value} class="underline" target="_blank">{value}</a>
	{/if}
{:else}
	{value}
{/if}
