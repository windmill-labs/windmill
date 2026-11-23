<script lang="ts">
	import { truncateHash } from '$lib/utils'
	import { Building } from 'lucide-svelte'
	import { Badge } from './common'
	import IconedResourceType from './IconedResourceType.svelte'

	interface Props {
		path: string
		hash?: string | undefined
		class?: string | undefined
		/** Color of the path text; a link passes `text-accent`. */
		textClass?: string
	}

	let {
		path,
		hash = undefined,
		class: clazz = undefined,
		textClass = 'text-primary'
	}: Props = $props()
</script>

<div class="flex space-x-2 items-center w-full shrink min-w-0 {clazz ?? ''}">
	{#if path.startsWith('hub/')}
		<div>
			<IconedResourceType width="20px" height="20px" name={path.split('/')[2]} silent={true} />
		</div>
		<span class="truncate text-xs {textClass}">{path}</span>
	{:else}
		<div class="center-center">
			<Building size={16} />
		</div>
		<span class="truncate text-xs {textClass}">{path}</span>
		{#if hash}
			<Badge>{truncateHash(hash)}</Badge>
		{/if}
	{/if}
</div>
