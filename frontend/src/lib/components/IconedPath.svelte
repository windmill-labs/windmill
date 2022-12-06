<script lang="ts">
	import { truncateHash } from '$lib/utils'
	import { Building } from 'svelte-lucide'
	import { Badge } from './common'
	import IconedResourceType from './IconedResourceType.svelte'

	export let path: string
	export let hash: string | undefined = undefined
</script>

<div class="flex space-x-2 items-center w-full shrink min-w-0 {$$props.class}">
	{#if path.startsWith('hub/')}
		<div>
			<IconedResourceType width="20px" height="20px" name={path.split('/')[2]} silent={true} />
		</div>
		<span class="text-sm truncate">{path}</span>
	{:else}
		<div class="center-center">
			<Building size="16px" />
		</div>
		<span class="text-sm truncate">{path}</span>
		{#if hash}
			<Badge>{truncateHash(hash)}</Badge>
		{/if}
	{/if}
</div>
