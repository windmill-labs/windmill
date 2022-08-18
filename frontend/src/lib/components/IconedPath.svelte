<script lang="ts">
	import IconedResourceType from './IconedResourceType.svelte'

	export let path: string
</script>

{#if path.startsWith('hub/')}
	<span class="inline-flex flex-row gap-x-2 items-center">
		<IconedResourceType name={path.split('/')[2]} silent={true} />
		<span>{path}</span>
	</span>
{:else}
	path
{/if}
