<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import IconedResourceType from './IconedResourceType.svelte'

	export let path: string
</script>

<div class="flex space-x-2 items-center">
	{#if path.startsWith('hub/')}
		<IconedResourceType name={path.split('/')[2]} silent={true} />
		<span>{path}</span>
	{:else}
		<Badge color="blue">Workspace</Badge>
		<span>{path}</span>
	{/if}
</div>
