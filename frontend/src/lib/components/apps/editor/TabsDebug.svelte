<script lang="ts">
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'

	export let tabs: any[] = []
	export let id: string

	export let isConditionalDebugMode: boolean = false

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	let isManuallySelected: boolean = false
	let selected: number | null = null
</script>

<button
	title={isConditionalDebugMode ? 'Debug conditions' : 'Debug tabs'}
	class={classNames(
		'text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
		isManuallySelected
			? 'bg-red-100 text-red-600 border-red-500 hover:bg-red-200 hover:text-red-800'
			: 'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
	)}
	on:click={() => dispatch('triggerInlineEditor')}
	on:pointerdown|stopPropagation
>
	<ButtonDropdown hasPadding={false}>
		<svelte:fragment slot="buttonReplacement">
			<div class="px-1">
				{#if isManuallySelected}
					<div>
						{#if selected === tabs.length - 1}
							{isConditionalDebugMode ? `Debug default condition` : `Debug tab ${selected + 1}`}
						{:else}
							{`Debugging ${isConditionalDebugMode ? 'condition' : 'tab'} ${(selected ?? 0) + 1}`}
						{/if}
					</div>
				{:else}
					{isConditionalDebugMode ? `Debug conditions` : `Debug tabs`}
				{/if}
			</div>
		</svelte:fragment>
		<svelte:fragment slot="items">
			{#each tabs ?? [] as { }, index}
				<MenuItem
					on:click={() => {
						$componentControl?.[id]?.setTab?.(index)
						selected = index
						isManuallySelected = true
					}}
				>
					<div
						class={classNames(
							'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
						)}
					>
						{#if index === tabs.length - 1}
							{isConditionalDebugMode ? `Debug default condition` : `Debug tab ${index + 1}`}
						{:else}
							{`Debug ${isConditionalDebugMode ? 'condition' : 'tab'} ${index + 1}`}
						{/if}
					</div>
				</MenuItem>
			{/each}
			<MenuItem
				on:click={() => {
					$componentControl?.[id]?.setTab?.(-1)
					selected = null
					isManuallySelected = false
				}}
			>
				<div
					class={classNames(
						'!text-red-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
					)}
				>
					{`Reset debug mode`}
				</div>
			</MenuItem>
		</svelte:fragment>
	</ButtonDropdown>
</button>
