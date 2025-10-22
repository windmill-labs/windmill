<script lang="ts">
	import { truncateHash } from '$lib/utils'
	import { Building } from 'lucide-svelte'
	import { Badge } from './common'
	import IconedResourceType from './IconedResourceType.svelte'

	interface Props {
		path: string
		hash?: string | undefined
		class?: string | undefined
	}

	let { path, hash = undefined, class: clazz = undefined }: Props = $props()
</script>

<div class="flex space-x-2 items-center w-full shrink min-w-0 {clazz ?? ''}">
	{#if path.startsWith('hub/')}
		<div>
			<IconedResourceType width="20px" height="20px" name={path.split('/')[2]} silent={true} />
		</div>
		<span class="text-sm truncate">{path}</span>
	{:else}
		<div class="center-center">
			<Building size={16} />
		</div>
		<span class="text-sm truncate">{path}</span>
		{#if hash}
			<Badge>{truncateHash(hash)}</Badge>
		{/if}
	{/if}
</div>
