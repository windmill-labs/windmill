<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { Bug } from 'lucide-svelte'

	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	interface Props {
		tabs?: any[]
		id: string
		isConditionalDebugMode?: boolean
		isSmall?: boolean
		isManuallySelected?: boolean
	}

	let {
		tabs = [],
		id,
		isConditionalDebugMode = false,
		isSmall = false,
		isManuallySelected = $bindable(false)
	}: Props = $props()
	let selected: number | null = $state(null)

	async function getItems() {
		return [
			...tabs.map((_, index) => ({
				displayName:
					index === tabs.length - 1
						? isConditionalDebugMode
							? `Debug default condition`
							: `Debug tab ${index + 1}`
						: `Debug ${isConditionalDebugMode ? 'condition' : 'tab'} ${index + 1}`,
				action: () => {
					$componentControl?.[id]?.setTab?.(index)
					selected = index
					isManuallySelected = true
				},
				type: 'action' as const
			})),
			{
				displayName: 'Reset debug mode',
				action: () => {
					$componentControl?.[id]?.setTab?.(-1)
					selected = null
					isManuallySelected = false
				},
				type: 'delete' as const
			}
		]
	}
</script>

{#key tabs}
	<Dropdown items={getItems} class="w-fit h-auto" usePointerDownOutside>
		{#snippet buttonReplacement()}
			<button
				title={isConditionalDebugMode ? 'Debug conditions' : 'Debug tabs'}
				class={classNames(
					'px-1 text-2xs font-bold w-fit h-full cursor-pointer rounded',
					isManuallySelected
						? 'hover:bg-red-200 hover:text-red-800'
						: 'text-blue-600 hover:bg-blue-300 hover:text-blue-800'
				)}
				onclick={() => dispatch('triggerInlineEditor')}
				onpointerdown={stopPropagation(bubble('pointerdown'))}
			>
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
			</button>
		{/snippet}
	</Dropdown>
{/key}
