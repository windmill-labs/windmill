<script lang="ts">
	import { NODE, type FlowNodeColorClasses } from '$lib/components/graph'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		label: string | undefined
		selectable: boolean
		id: string | undefined
		onTop?: boolean
		children?: import('svelte').Snippet<[any]>
		outputPickerVisible?: boolean
		className?: string
		previewButton?: import('svelte').Snippet
		colorClasses: FlowNodeColorClasses
	}

	let {
		label,
		selectable,
		id,
		onTop = false,
		children,
		className,
		previewButton,
		colorClasses
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

<div class="relative">
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={classNames(
			'w-full flex relative rounded-md drop-shadow-base',
			colorClasses.bg,
			onTop ? 'z-[901]' : '',
			className
		)}
		style="width: {NODE.width}px; height: {NODE.height}px;"
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
		<div class={twMerge('absolute rounded-md inset-0', selectable ? 'cursor-pointer' : '')}> </div>
		{@render children?.({ hover })}
	</div>

	{@render previewButton?.()}
</div>
