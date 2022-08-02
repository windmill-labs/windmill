<script lang="ts">
	import type { SchemaProperty } from '$lib/common'
	import { Badge } from 'flowbite-svelte'

	export let property: SchemaProperty
</script>

<Badge color="blue">{property.type?.toUpperCase() ?? 'ANY'}</Badge>
{#if property.format}
	<Badge color="green">{property.format?.toUpperCase()}</Badge>
{/if}
{#if property.contentEncoding}
	<Badge color="indigo">{property.contentEncoding}</Badge>
{/if}
