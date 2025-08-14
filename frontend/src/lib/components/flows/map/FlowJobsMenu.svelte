<script lang="ts">
	import { Menu, Menubar, MenuItem, MeltButton } from '$lib/components/meltComponents'
	import { ListFilter, Lock, LockOpen } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { twMerge } from 'tailwind-merge'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import type { onSelectedIteration } from '$lib/components/graph/graphBuilder.svelte'
	import { untrack } from 'svelte'

	interface Props {
		id: string
		moduleId: string
		flowJobs: string[] | undefined
		flowJobsSuccess: (boolean | undefined)[] | undefined
		selected: number
		selectedManually: boolean | undefined
		onSelectedIteration: onSelectedIteration
		showIcon?: boolean
	}

	let {
		id,
		flowJobs,
		flowJobsSuccess,
		selected,
		selectedManually,
		onSelectedIteration,
		moduleId,
		showIcon = true
	}: Props = $props()

	let filter: number | undefined = $state(undefined)
	function onKeydown(event: KeyboardEvent) {
		if (
			event.key === 'Enter' &&
			filter != undefined &&
			flowJobs &&
			filter < flowJobs.length &&
			filter > 0
		) {
			event.preventDefault()
			onSelectedIteration({
				index: filter - 1,
				id: flowJobs[filter - 1],
				manuallySet: true,
				moduleId: moduleId
			})
			menu?.close()
		}
	}

	let buttonHover = $state(false)
	let menu: Menu | undefined = $state(undefined)

	let items: { id: string; success: boolean | undefined; index: number }[] = $state([])
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

	let isOpen = $state(false)

	$effect(() => {
		isOpen && flowJobs && untrack(() => updateItems())
	})
</script>

{#if selectedManually && flowJobsSuccess?.some((x) => x == undefined || x == null)}
	<Popover class="absolute top-1.5 right-12 cursor-pointer">
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onmouseenter={() => (buttonHover = true)}
			onmouseleave={() => (buttonHover = false)}
			onclick={(e) => {
				buttonHover = false
				onSelectedIteration({ manuallySet: false, moduleId: moduleId })
			}}
		>
			{#if buttonHover}
				<LockOpen class="text-primary" size={14} />
			{:else}
				<Lock class="text-primary" size={12} />
			{/if}
		</div>
		{#snippet text()}
			<span>
				The iteration picked is locked. Click to unlock to pick automatically new iterations.
			</span>
		{/snippet}
	</Popover>
{/if}

<Menubar>
	{#snippet children({ createMenu })}
		<Menu
			on:open={() => {
				isOpen = true
				updateItems()
			}}
			on:close={() => {
				isOpen = false
			}}
			{createMenu}
			placement="bottom"
			bind:this={menu}
			usePointerDownOutside
		>
			{#snippet triggr({ trigger })}
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
					{#if showIcon}
						<ListFilter size={15} />
					{/if}
				</MeltButton>
			{/snippet}

			{#snippet children({ item })}
				<div class="flex flex-col px-1">
					<input type="number" bind:value={filter} onkeydown={onKeydown} />

					<div class="max-h-[300px]">
						{#key items}
							<VirtualList height={300} width="100%" itemCount={items.length} itemSize={24}>
								{#snippet header()}{/snippet}
								{#snippet footer()}{/snippet}
								{#snippet children({ index: idx, style })}
									<div {style}>
										<MenuItem
											class={twMerge(
												'text-primary text-xs w-full text-left py-1 pl-2 hover:bg-surface-hover whitespace-nowrap flex flex-row gap-2 items-center',
												items[idx].success == false ? 'text-red-400' : '',
												'data-[highlighted]:bg-surface-hover'
											)}
											onClick={() => {
												onSelectedIteration({
													moduleId: moduleId,
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
								{/snippet}
							</VirtualList>
						{/key}

						<!-- {#each flowJobs ?? [] as id, idx (id)}
								{#if filter == undefined || (idx + 1).toString().includes(filter.toString())}

								{/if}
							{/each} -->
					</div>
				</div>
			{/snippet}
		</Menu>
	{/snippet}
</Menubar>
