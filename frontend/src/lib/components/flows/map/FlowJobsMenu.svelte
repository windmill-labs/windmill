<script lang="ts">
	import { Menu, Menubar, MenuItem, MeltButton } from '$lib/components/meltComponents'
	import { createEventDispatcher } from 'svelte'
	import { ListFilter, Lock, LockOpen } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import VirtualList from 'svelte-tiny-virtual-list'

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

	let items: { id: string; success: boolean | undefined; index: number }[] = []
	function updateItems() {
		items = (flowJobs ?? [])
			.map((v, i) => {
				return {
					id: v,
					success: flowJobsSuccess?.[i],
					index: i
				}
			})
			.filter((v) => {
				return filter == undefined || (v.index + 1).toString().includes(filter.toString())
			})
	}

	let isOpen = false

	$: isOpen && flowJobs && updateItems()
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
	<Menu
		on:open={() => {
			isOpen = true
			updateItems()
		}}
		on:close={() => {
			isOpen = false
		}}
		{createMenu}
		let:item
		placement="bottom"
		bind:this={menu}
		usePointerDownOutside
	>
		<svelte:fragment slot="trigger" let:trigger>
			<MeltButton
				title="Pick an iteration"
				id={`flow-editor-iteration picker-${id}`}
				type="button"
				class={twMerge(
					'text-xs bg-surface border-[1px] border-gray-300 dark:border-gray-500 focus:outline-none',
					'hover:bg-surface-hover focus:ring-4 focus:ring-surface-selected font-medium rounded-sm w-[40px] gap-1 h-[20px]',
					'flex items-center justify-center',
					flowJobsSuccess?.[selected] == false ? 'text-red-400' : 'text-secondary'
				)}
				meltElement={trigger}
			>
				#{selected == -1 ? '?' : selected + 1}
				<ListFilter size={15} />
			</MeltButton>
		</svelte:fragment>

		<div class="flex flex-col px-1">
			<input type="number" bind:value={filter} on:keydown={onKeydown} />

			<div class="max-h-[300px]">
				{#key items}
					<VirtualList height={300} width="100%" itemCount={items.length} itemSize={24}>
						<div slot="item" let:index={idx} let:style {style}>
							<MenuItem
								class={twMerge(
									'text-primary text-xs w-full text-left py-1 pl-2 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center',
									items[idx].success == false ? 'text-red-400' : '',
									'data-[highlighted]:bg-surface-hover'
								)}
								on:click={() => {
									dispatch('selectedIteration', {
										index: items[idx].index,
										id: items[idx].id,
										manuallySet: true
									})
									menu?.close()
								}}
								{item}
							>
								#{items[idx].index + 1}
							</MenuItem>
						</div>
					</VirtualList>
				{/key}

				<!-- {#each flowJobs ?? [] as id, idx (id)}
						{#if filter == undefined || (idx + 1).toString().includes(filter.toString())}

						{/if}
					{/each} -->
			</div>
		</div></Menu
	>
</Menubar>
