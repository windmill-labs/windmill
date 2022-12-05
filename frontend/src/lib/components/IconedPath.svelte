<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import IconedResourceType from './IconedResourceType.svelte'

	export let path: string
</script>

<div class="flex space-x-2 items-center">
	{#if path.startsWith('hub/')}
		<div>
			<IconedResourceType width="20px" height="20px" name={path.split('/')[2]} silent={true} />
		</div>
		<span class="text-sm">{path}</span>
	{:else}
		<Badge color="blue">Workspace</Badge>
		<span class="text-sm">{path}</span>
	{/if}
</div>
