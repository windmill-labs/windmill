<script lang="ts">
	import { Menu, Menubar } from '$lib/components/meltComponents/index'
	import { melt } from '@melt-ui/svelte'
	import { createEventDispatcher } from 'svelte'
	import { ListFilter, Lock, LockOpen } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'

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
			dispatch('selectedIteration', {
				index: filter - 1,
				id: flowJobs[filter - 1],
				manuallySet: true
			})
			menu?.close()
		}
	}

	let buttonHover = false
	let menu: Menu | undefined = undefined
</script>

{#if selectedManually && flowJobsSuccess?.some((x) => x == undefined || x == null)}
	<Popover class="absolute top-1.5 right-12 cursor-pointer">
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			on:mouseenter={() => (buttonHover = true)}
			on:mouseleave={() => (buttonHover = false)}
			on:click={(e) => {
				buttonHover = false
				dispatch('selectedIteration', { manuallySet: false })
			}}
		>
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

<Menubar let:createMenu>
	<Menu {createMenu} let:item placement="bottom" bind:this={menu}>
		<svelte:fragment slot="trigger" let:trigger>
			<button
				title="Pick an iteration"
				id={`flow-editor-iteration picker-${id}`}
				type="button"
				class={twMerge(
					'text-xs bg-surface border-[1px] border-gray-300 dark:border-gray-500 focus:outline-none',
					'hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-sm w-[40px] gap-1 h-[20px]',
					'flex items-center justify-center',
					flowJobsSuccess?.[selected] == false ? 'text-red-400' : 'text-secondary'
				)}
				use:melt={trigger}
			>
				#{selected == -1 ? '?' : selected + 1}
				<ListFilter size={15} />
			</button>
		</svelte:fragment>

		<div class="flex flex-col px-1">
			<input type="number" bind:value={filter} on:keydown={onKeydown} />

			<div class="overflow-y-auto max-h-[300px]">
				{#each flowJobs ?? [] as id, idx (id)}
					{#if filter == undefined || (idx + 1).toString().includes(filter.toString())}
						<button
							class={twMerge(
								'text-primary text-xs w-full text-left py-1 pl-2 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center',
								flowJobsSuccess?.[idx] == false ? 'text-red-400' : '',
								'data-[highlighted]:bg-surface-hover'
							)}
							on:click={() => {
								dispatch('selectedIteration', { index: idx, id, manuallySet: true })
							}}
							role="menuitem"
							tabindex="-1"
							use:melt={item}
						>
							#{idx + 1}
						</button>
					{/if}
				{/each}
			</div>
		</div>
	</Menu>
</Menubar>
