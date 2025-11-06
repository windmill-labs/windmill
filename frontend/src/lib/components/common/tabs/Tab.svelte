<script lang="ts">
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	const dispatch = createEventDispatcher<{ onpointerdown: any }>()

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		value: string
		label?: string
		icon?: any | undefined
		class?: string
		style?: string
		selectedClass?: string
		selectedStyle?: string
		id?: string | undefined
		active?: boolean | undefined
		exact?: boolean
		otherValues?: string[]
		disabled?: boolean
		/**
		 * @deprecated TODO : Re-organize workspace settings so we don't have so many tabs
		 */
		small?: boolean
		extra?: import('svelte').Snippet
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		value,
		label,
		icon = undefined,
		class: c = '',
		style = '',
		selectedClass = '',
		selectedStyle = '',
		id = undefined,
		active = false,
		exact = false,
		otherValues = [],
		disabled = false,
		small = false,
		extra = undefined
	}: Props = $props()
	const { selected, update, hashNavigation } = getContext<TabsContext>('Tabs')

	function getIsSelectedFn(exact: boolean, otherValues: string[]) {
		if (otherValues.length > 0) {
			return (selected: string) => selected == value || otherValues.includes(selected)
		} else if (exact) {
			return (selected: string) => selected == value
		} else {
			return (selected: string) => selected?.startsWith(value)
		}
	}

	let isSelectedFn = $derived(getIsSelectedFn(exact, otherValues))

	let isSelected = $derived(isSelectedFn($selected))
</script>

<button
	use:triggerableByAI={{
		id: aiId,
		description: aiDescription,
		callback: () => {
			if (hashNavigation) {
				window.location.hash = value
			} else {
				update(value)
			}
		}
	}}
	class={twMerge(
		'border-b-2 border-border-light py-1 cursor-pointer transition-all z-10 ease-linear text-primary font-normal text-xs',
		isSelected
			? 'text-emphasis font-semibold border-border-normal'
			: 'border-opacity-0 hover:border-opacity-100',
		small ? 'px-1' : 'px-2',
		c,
		isSelected ? selectedClass : '',
		disabled ? 'cursor-not-allowed text-disabled' : ''
	)}
	style={`${style} ${isSelected ? selectedStyle : ''}`}
	onclick={() => {
		if (hashNavigation) {
			window.location.hash = value
		} else {
			update(value)
		}
	}}
	onpointerdown={(event) => {
		event.stopPropagation()
		dispatch('onpointerdown', event)
	}}
	{disabled}
	{id}
>
	<div
		class={twMerge(
			active ? 'bg-surface-accent-selected text-accent rounded-md' : '',
			'flex gap-2 items-center my-1 px-2'
		)}
	>
		{#if icon}
			{@const IconComponent = icon}
			<IconComponent size={14} />
		{/if}
		{#if label}
			{label}
		{/if}
		{@render extra?.()}
	</div>
</button>
