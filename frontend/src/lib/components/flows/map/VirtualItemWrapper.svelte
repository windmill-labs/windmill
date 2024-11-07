<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'

	export let label: string | undefined
	export let selectable: boolean
	export let selected: boolean
	export let id: string | undefined
	export let onTop: boolean = false
	export let bgColor: string

	const dispatch = createEventDispatcher<{
		insert: {
			script?: { path: string; summary: string; hash: string | undefined }
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger' | 'move'
			modules: FlowModule[]
			index: number
		}
		select: string
	}>()
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={classNames(
		'w-full flex relative rounded-sm',
		selectable ? 'cursor-pointer' : '',
		selected ? 'outline outline-offset-1 outline-2  outline-gray-600 dark:outline-gray-400' : '',
		onTop ? 'z-[901]' : ''
	)}
	style="width: 275px; max-height: 34px; background-color: {bgColor} !important;"
	on:click={() => {
		if (selectable) {
			if (id) {
				dispatch('select', id)
			} else {
				dispatch('select', label || label || '')
			}
		}
	}}
	title={label ? label + ' ' : ''}
	id={`flow-editor-virtual-${encodeURIComponent(label || label || '')}`}><slot /></div
>
