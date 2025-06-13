<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		label: string | undefined
		selectable: boolean
		selected: boolean
		id: string | undefined
		onTop?: boolean
		bgColor: string
		bgHoverColor?: string
		children?: import('svelte').Snippet<[any]>
	}

	let {
		label,
		selectable,
		selected,
		id,
		onTop = false,
		bgColor,
		bgHoverColor = '',
		children
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
	class={classNames(
		'w-full flex relative rounded-sm',
		selectable ? 'cursor-pointer active:outline active:outline-2' : '',
		selected ? 'outline outline-2' : '',
		onTop ? 'z-[901]' : '',
		'outline-offset-1 outline-gray-600 dark:outline-gray-400'
	)}
	style="width: 275px; max-height: 38px; background-color: {hover && bgHoverColor && selectable
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
	>{@render children?.({ hover })}</div
>
