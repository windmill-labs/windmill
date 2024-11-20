<script lang="ts">
	import Menu from '$lib/components/common/menu/MenuV2.svelte'
	import { createEventDispatcher } from 'svelte'
	import { ListFilter, Lock, LockOpen } from 'lucide-svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import Popover from '$lib/components/Popover.svelte'

	const dispatch = createEventDispatcher()

	export let id: string
	export let flowJobs: string[] | undefined
	export let flowJobsSuccess: (boolean | undefined)[] | undefined
	export let selected: number
	export let selectedManually: boolean | undefined

	let filter: number | undefined = undefined
	function onKeydown(event: KeyboardEvent) {
		if (
			event.key === 'Enter' &&
			filter != undefined &&
			flowJobs &&
			filter < flowJobs.length &&
			filter > 0
		) {
			event.preventDefault()
			dispatch('selectedIteration', { index: filter - 1, id: flowJobs[filter - 1], manuallySet: true })
		}
	}

	let buttonHover = false
</script>

{#if selectedManually && flowJobsSuccess?.some((x) => x == undefined || x == null)}
	<Popover class="absolute top-1.5 right-12 cursor-pointer">
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div on:mouseenter={() => (buttonHover = true)} on:mouseleave={() => (buttonHover = false)} on:click={(e) => {
				buttonHover = false
				dispatch('selectedIteration', { manuallySet: false })}
			}>
			{#if buttonHover}
				<LockOpen class="text-primary" size={14} />
			{:else}
				<Lock class="text-primary" size={12} />
			{/if}
		</div>
		<span slot="text">
			The iteration picked is locked. Click to unlock to pick automatically new iterations.
		</span>
	</Popover>
{/if}

<Menu  maxHeight={300}>
	<div slot="trigger">
		<button
			title="Pick an iteration"
			id={`flow-editor-iteration picker-${id}`}
			type="button"
			class=" text-xs bg-surface border-[1px] border-gray-300 dark:border-gray-500 focus:outline-none
		hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-sm w-[40px] gap-1 h-[20px]
		flex items-center justify-center {flowJobsSuccess?.[selected] == false
				? 'text-red-400'
				: 'text-secondary'}"
		>
			#{selected == -1 ? '?' : selected + 1}
			<ListFilter size={15} />
		</button>
	</div>

	<MenuItem disabled>
		<input type="number" bind:value={filter} on:keydown={onKeydown} />
	</MenuItem>
	{#each flowJobs ?? [] as id, idx (id)}
		{#if filter == undefined || (idx + 1).toString().includes(filter.toString())}
			<MenuItem>
				<button
					class="text-primary text-xs w-full text-left py-1 pl-2 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center {flowJobsSuccess?.[
						idx
					] == false
						? 'text-red-400'
						: ''}"
					on:pointerdown={() => {
						//close()
						dispatch('selectedIteration', { index: idx, id, manuallySet: true })
					}}
					role="menuitem"
					tabindex="-1"
				>
					#{idx + 1}
				</button>
			</MenuItem>
		{/if}
	{/each}
</Menu>
