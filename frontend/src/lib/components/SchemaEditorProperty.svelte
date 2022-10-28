<script lang="ts">
	import type { SchemaProperty } from '$lib/common'
	import Badge from './common/badge/Badge.svelte'

	export let property: SchemaProperty
</script>

<div class="flex flex-row flex-wrap gap-1">
	<Badge color="blue">{property.type?.toUpperCase() ?? 'ANY'}</Badge>
	{#if property.format}
		<Badge color="green">{property.format?.toUpperCase()}</Badge>
	{/if}
	{#if property.contentEncoding}
		<Badge color="indigo">{property.contentEncoding?.toUpperCase()}</Badge>
	{/if}
</div>
