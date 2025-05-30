<script lang="ts">
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let value: string
	export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'sm'
	let c = ''
	export { c as class }
	export let style = ''
	export let selectedClass = ''
	export let selectedStyle = ''
	export let id: string | undefined = undefined
	export let active: boolean | undefined = false
	export let exact = false
	export let otherValues: string[] = []

	export let disabled: boolean = false
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

	$: isSelectedFn = getIsSelectedFn(exact, otherValues)

	$: isSelected = isSelectedFn($selected)

	const fontSizeClasses = {
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	}
</script>

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
	on:click={() => {
		if (hashNavigation) {
			window.location.hash = value
		} else {
			update(value)
		}
	}}
	on:pointerdown|stopPropagation
	{disabled}
	{id}
>
	<div class={twMerge(active ? 'bg-blue-50 text-blue-800 rounded-md ' : '', 'px-2 ')}>
		<slot />
	</div>
</button>
