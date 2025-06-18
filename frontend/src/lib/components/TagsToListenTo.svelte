<script lang="ts">
	import { X, Plus, Trash } from 'lucide-svelte'
	import { Button } from './common'
	import { superadmin } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import { defaultTags, nativeTags } from './worker_group'
	import Select from './select/Select.svelte'
	import { safeSelectItems } from './select/utils.svelte'

	const dispatch = createEventDispatcher()
	type Props = {
		worker_tags: string[]
		customTags: string[] | undefined
		disabled?: boolean
	}
	let {
		worker_tags = $bindable([]),
		customTags = $bindable([]),
		disabled = $bindable(false)
	}: Props = $props()
	let newTag = $state('')
	let createdTags: string[] = $state([])
</script>

<div class="flex gap-2 gap-y-2 flex-wrap pb-3">
	{#if worker_tags?.length === 0}
		<div class="text-xs text-secondary">No tags selected</div>
	{/if}

	{#each worker_tags as tag}
		<div
			class="flex items-center gap-1 px-2 py-1 rounded-full border border-primary text-2xs text-primary bg-surface-primary"
		>
			<span>{tag}</span>
			{#if $superadmin && !disabled}
				<Button
					class="p-1 rounded-full hover:bg-surface-hover transition"
					aria-label="Remove tag"
					on:click={() => {
						worker_tags = worker_tags?.filter((t) => t !== tag) ?? []
						dispatch('dirty')
						dispatch('deletePriorityTag', tag)
					}}
				>
					<X size={12} />
				</Button>
			{/if}
		</div>
	{/each}
</div>

{#if $superadmin}
	<div class="max-w-md space-y-2">
		<Select
			items={safeSelectItems(
				[...(customTags ?? []), ...createdTags, ...defaultTags, ...nativeTags].filter(
					(x) => !worker_tags?.includes(x)
				)
			)}
			{disabled}
			bind:value={newTag}
			onFocus={() => dispatch('focus')}
			onCreateItem={(c) => (createdTags.push(c), (createdTags = [...createdTags]), (newTag = c))}
			createText="Press Enter to use this tag"
		/>

		<div class="flex gap-2">
			<Button
				variant="contained"
				color="light"
				size="xs"
				startIcon={{ icon: Plus }}
				disabled={disabled || newTag == '' || worker_tags?.includes(newTag)}
				on:click={() => {
					worker_tags = [...(worker_tags ?? []), newTag.replaceAll(' ', '_')]
					newTag = ''
					dispatch('dirty')
				}}
			>
				Add tag
			</Button>
			<Button
				variant="contained"
				color="red"
				size="xs"
				startIcon={{ icon: Trash }}
				disabled={disabled || worker_tags.length === 0}
				on:click={() => {
					worker_tags = worker_tags.filter((tag) => {
						dispatch('deletePriorityTag', tag)
						return false
					})
					dispatch('dirty')
				}}
			>
				Remove all selected tags
			</Button>
		</div>
	</div>
{/if}
