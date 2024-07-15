<script lang="ts">
	import { Menu } from '$lib/components/common'
	import { createEventDispatcher } from 'svelte'
	import { ListFilter } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	export let open: boolean | undefined = undefined
	export let index: number
	export let flowJobs: string[] | undefined
	export let flowJobsSuccess: (boolean | undefined)[] | undefined
	export let selected: number

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
			dispatch('selectedIteration', { index: filter - 1, id: flowJobs[filter - 1] })
		}
	}
</script>

<Menu
	transitionDuration={0}
	pointerDown
	bind:show={open}
	noMinW
	placement="bottom-center"
	let:close
>
	<button
		title="Pick an interation"
		slot="trigger"
		id={`flow-editor-iteration picker-${index}`}
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
	<div id="flow-editor-insert-module">
		<div class="font-mono divide-y text-xs w-full text-secondary max-h-[200px] overflow-auto">
			<input autofocus type="number" bind:value={filter} on:keydown={onKeydown} />
			{#each flowJobs ?? [] as id, idx (id)}
				{#if filter == undefined || (idx + 1).toString().includes(filter.toString())}
					<button
						class="w-full text-left py-1 pl-2 min-w-20 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center {flowJobsSuccess?.[
							idx
						] == false
							? 'text-red-400'
							: ''}"
						on:pointerdown={() => {
							close()
							dispatch('selectedIteration', { index: idx, id })
						}}
						role="menuitem"
						tabindex="-1"
					>
						#{idx + 1}
					</button>
				{/if}
			{/each}
		</div>
	</div>
</Menu>
