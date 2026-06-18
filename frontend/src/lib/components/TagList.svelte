<script lang="ts">
	import Badge from './common/badge/Badge.svelte'
	import Popover from './meltComponents/Popover.svelte'

	interface Props {
		tags: string[]
		maxVisible?: number
		class?: string | undefined
	}

	let { tags, maxVisible = undefined, class: clazz = undefined }: Props = $props()

	const visibleTags = $derived(maxVisible ? tags.slice(0, maxVisible) : tags)
	const extraTags = $derived(maxVisible ? tags.slice(maxVisible) : [])
	const hasExtraTags = $derived(extraTags.length > 0)
</script>

{#if tags.length > 0}
	<div class="flex items-center gap-1 min-w-0 {clazz} w-full">
		<!-- Display visible tags -->
		{#each visibleTags as tag (tag)}
			<Badge color="blue" small wrapperClass="shrink min-w-0" class="truncate" title={tag}>
				<span class="min-w-0 truncate">{tag}</span>
			</Badge>
		{/each}

		<!-- Display +n badge with popover for extra tags -->
		{#if hasExtraTags}
			<Popover
				floatingConfig={{
					strategy: 'absolute',
					placement: 'bottom-start'
				}}
				contentClasses="border border-light rounded-lg shadow-lg p-4 surface-tertiary max-w-xs"
				openOnHover
				debounceDelay={150}
			>
				{#snippet trigger()}
					<Badge color="blue" small clickable>
						+{extraTags.length}
					</Badge>
				{/snippet}
				{#snippet content()}
					<div class="flex flex-wrap gap-1">
						{#each extraTags as tag (tag)}
							<Badge color="blue" verySmall class="max-w-20 truncate" title={tag}>{tag}</Badge>
						{/each}
					</div>
				{/snippet}
			</Popover>
		{/if}
	</div>
{:else}
	<span class="text-secondary text-xs">No tags</span>
{/if}
