<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let label: string | undefined
	export let selectable: boolean
	export let selected: boolean
	export let id: string | undefined
	export let onTop: boolean = false
	export let bgColor: string
	export let bgHoverColor: string = ''
	export let outputPickerVisible: boolean = false

	const dispatch = createEventDispatcher<{
		insert: {
			script?: { path: string; summary: string; hash: string | undefined }
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger' | 'move'
			modules: FlowModule[]
			index: number
		}
		select: string
	}>()

	let hover: boolean = false
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={classNames('w-full flex relative rounded-sm', onTop ? 'z-[901]' : '')}
	style="width: 275px; max-height: 34px; background-color: {hover && bgHoverColor && selectable
		? bgHoverColor
		: bgColor};"
	on:pointerdown={() => {
		if (selectable) {
			dispatch('select', id || label || '')
		}
	}}
	on:mouseenter={() => {
		hover = true
	}}
	on:mouseleave={() => {
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
	></div>
	<slot {hover} />
</div>
