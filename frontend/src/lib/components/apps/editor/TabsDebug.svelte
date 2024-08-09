<script lang="ts">
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { Bug } from 'lucide-svelte'

	export let tabs: any[] = []
	export let id: string

	export let isConditionalDebugMode: boolean = false
	export let isSmall = false

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	export let isManuallySelected: boolean = false
	let selected: number | null = null
</script>

<button
	title={isConditionalDebugMode ? 'Debug conditions' : 'Debug tabs'}
	class={classNames(
		'text-2xs font-bold w-fit h-full cursor-pointer rounded',
		isManuallySelected
			? 'hover:bg-red-200 hover:text-red-800'
			: ' hover:text-indigo-800 hover:bg-indigo-300'
	)}
	on:click={() => dispatch('triggerInlineEditor')}
	on:pointerdown|stopPropagation
>
	<ButtonDropdown hasPadding={false}>
		<svelte:fragment slot="buttonReplacement">
			<div class="px-1">
				{#if isManuallySelected}
					<div class="whitespace-nowrap">
						{#if selected === tabs.length - 1}
							{#if isSmall}
								{isConditionalDebugMode ? `df` : `t ${selected + 1}`}
							{:else}
								{isConditionalDebugMode ? `Debug default condition` : `Debug tab ${selected + 1}`}
							{/if}
						{:else if isSmall}
							{`${isConditionalDebugMode ? 'c' : 't'} ${(selected ?? 0) + 1}`}
						{:else}
							{`Debugging ${isConditionalDebugMode ? 'condition' : 'tab'} ${
								(selected ?? 0) + 1
							}`}{/if}
					</div>
				{:else if isSmall}<Bug size={11} />{:else}
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
							'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
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
						'!text-red-600 dark:!text-red-400 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
					)}
				>
					{`Reset debug mode`}
				</div>
			</MenuItem>
		</svelte:fragment>
	</ButtonDropdown>
</button>
