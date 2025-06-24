<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		label: string | undefined
		selectable: boolean
		selected: boolean
		id: string | undefined
		onTop?: boolean
		bgColor: string
		bgHoverColor?: string
		children?: import('svelte').Snippet<[any]>
		outputPickerVisible?: boolean
		className?: string
	}

	let {
		label,
		selectable,
		selected,
		id,
		onTop = false,
		bgColor,
		bgHoverColor = '',
		children,
		outputPickerVisible = false,
		className
	}: Props = $props()

	const dispatch = createEventDispatcher<{
		insert: {
			script?: { path: string; summary: string; hash: string | undefined }
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger' | 'move'
			modules: FlowModule[]
			index: number
		}
		select: string
	}>()

	let hover: boolean = $state(false)
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class={classNames('w-full flex relative rounded-sm', onTop ? 'z-[901]' : '', className)}
	style="width: 275px; max-height: 34px; background-color: {hover && bgHoverColor && selectable
		? bgHoverColor
		: bgColor};"
	onpointerdown={() => {
		if (selectable) {
			dispatch('select', id || label || '')
		}
	}}
	onmouseenter={() => {
		hover = true
	}}
	onmouseleave={() => {
		hover = false
	}}
	title={label ? label + ' ' : ''}
	id={`flow-editor-virtual-${encodeURIComponent(label || label || '')}`}
>
	<div
		class={twMerge(
			'absolute  outline-gray-600 dark:outline-gray-400 rounded-sm',
			selected ? 'outline outline-2' : '',
			selectable ? 'cursor-pointer active:outline active:outline-2' : ''
		)}
		style={`width: 275px; height: ${outputPickerVisible ? '50px' : '34px'};`}
	>
	</div>
	{@render children?.({ hover })}
</div>
