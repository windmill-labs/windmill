<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import TriggerableByAI from '$lib/components/TriggerableByAI.svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		value: string
		size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
		class?: string
		style?: string
		selectedClass?: string
		selectedStyle?: string
		id?: string | undefined
		active?: boolean | undefined
		exact?: boolean
		otherValues?: string[]
		disabled?: boolean
		children?: import('svelte').Snippet
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		value,
		size = 'sm',
		class: c = '',
		style = '',
		selectedClass = '',
		selectedStyle = '',
		id = undefined,
		active = false,
		exact = false,
		otherValues = [],
		disabled = false,
		children
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

	const fontSizeClasses = {
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	}
</script>

<TriggerableByAI
	id={aiId}
	description={aiDescription}
	onTrigger={() => {
		if (hashNavigation) {
			window.location.hash = value
		} else {
			update(value)
		}
	}}
>
	<button
		class={twMerge(
			'border-b-2 py-1 px-2 cursor-pointer transition-all z-10 ease-linear font-normal text-tertiary',
			isSelected
				? 'wm-tab-active font-main'
				: 'border-gray-300 dark:border-gray-600 border-opacity-0 hover:border-opacity-100 ',
			fontSizeClasses[size],
			c,
			isSelected ? selectedClass : '',
			disabled ? 'cursor-not-allowed text-tertiary' : ''
		)}
		style={`${style} ${isSelected ? selectedStyle : ''}`}
		onclick={() => {
			if (hashNavigation) {
				window.location.hash = value
			} else {
				update(value)
			}
		}}
		onpointerdown={stopPropagation(bubble('pointerdown'))}
		{disabled}
		{id}
	>
		<div class={twMerge(active ? 'bg-blue-50 text-blue-800 rounded-md ' : '', 'px-2 ')}>
			{@render children?.()}
		</div>
	</button>
</TriggerableByAI>
