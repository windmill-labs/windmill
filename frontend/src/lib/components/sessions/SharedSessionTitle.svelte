<!--
@component
A shared session's name in the page header's breadcrumb, followed by who shared it and when.
-->
<script lang="ts">
	import { Globe, Share2 } from 'lucide-svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { userColorClass, userInitials } from '$lib/components/userInitials'
	import { displayDate } from '$lib/utils'

	let {
		name,
		author,
		sharedAt,
		isPublic = false
	}: { name: string; author?: string; sharedAt?: string; isPublic?: boolean } = $props()
</script>

<span class="truncate text-xs font-normal text-emphasis" title={name}>{name}</span>
{#if author}
	<Tooltip class="ml-2 inline-flex shrink-0 items-center">
		<span class="inline-flex items-center gap-1">
			<span class="inline-flex items-center gap-0.5 text-2xs font-normal text-hint">
				<Share2 size={10} aria-hidden="true" />Shared by
			</span>
			<span
				class="inline-flex h-4 w-4 items-center justify-center rounded-full text-[8px] font-semibold text-white {userColorClass(
					author
				)}"
				aria-label={author}
			>
				{userInitials(author)}
			</span>
		</span>
		{#snippet text()}
			<div class="flex flex-col gap-0.5">
				<span>Shared by <span class="font-medium">{author}</span></span>
				{#if sharedAt}
					<span class="opacity-80">{displayDate(sharedAt)}</span>
				{/if}
			</div>
		{/snippet}
	</Tooltip>
{/if}
{#if isPublic}
	<span
		class="ml-2 inline-flex shrink-0 items-center gap-0.5 text-2xs font-normal text-hint"
		title="Anyone with the link can read it"
	>
		<Globe size={10} aria-hidden="true" />Public
	</span>
{/if}
