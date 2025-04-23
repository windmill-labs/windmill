<script lang="ts">
	import { X } from 'lucide-svelte'
	import { Button } from './common'
	import { Plus } from 'lucide-svelte'
	import { superadmin } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { defaultTags, nativeTags } from './worker_group'

	const dispatch = createEventDispatcher()

	export let worker_tags: string[] = []
	export let customTags: string[] = []
	export let disabled = false

	let newTag = ''
	let createdTags: string[] = []
</script>

<div class="flex gap-3 gap-y-2 flex-wrap pb-2">
	{#if worker_tags?.length == 0}
		<div class="text-xs text-secondary">No tags selected</div>
	{/if}
	{#each worker_tags as tag}
		<div class="flex gap-0.5 items-center"
			><div class="text-2xs p-1 rounded border text-primary">{tag}</div>
			{#if $superadmin && !disabled}
				<button
					class={'z-10 rounded-full p-1 duration-200 hover:bg-gray-200'}
					aria-label="Remove item"
					on:click|preventDefault|stopPropagation={() => {
						worker_tags = worker_tags?.filter((t) => t != tag) ?? []
						dispatch('dirty')
						dispatch('deletePriorityTag', tag)
					}}
				>
					<X size={12} />
				</button>
			{/if}</div
		>
	{/each}
</div>
{#if $superadmin}
	<div class="max-w-md">
		<AutoComplete
			noInputStyles
			items={[...(customTags ?? []), ...createdTags, ...defaultTags, ...nativeTags].filter(
				(x) => !worker_tags?.includes(x)
			)}
			{disabled}
			bind:selectedItem={newTag}
			hideArrow={true}
			inputClassName={'flex !font-gray-600 !font-primary !bg-surface-primary"'}
			dropdownClassName="!text-sm !py-2 !rounded-sm  !border-gray-200 !border !shadow-md"
			className="w-full !font-gray-600 !font-primary !bg-surface-primary"
			onFocus={() => {
				dispatch('focus')
			}}
			create
			onCreate={(c) => {
				createdTags.push(c)
				createdTags = [...createdTags]
				return c
			}}
			createText="Press enter to use this tag"
		/>

		<div class="mt-1"></div>
		<div class="flex">
			<Button
				variant="contained"
				color="blue"
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
		</div>
	</div>
{/if}
