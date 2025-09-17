<script lang="ts">
	import { NodeResizer } from '@xyflow/svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		data: {
			text: string
			color: string
			onUpdate?: (text: string) => void
			onDelete?: () => void
			onSizeChange?: (size: { width: number; height: number }) => void
		}
		selected?: boolean
		dragging?: boolean
	}

	let { data, selected = false, dragging = false }: Props = $props()

	let textareaElement: HTMLTextAreaElement | undefined = $state(undefined)

	function handleTextChange(event: Event) {
		const target = event.target as HTMLTextAreaElement
		// Call the update callback
		data.onUpdate?.(target.value)

		// Auto-resize textarea
		target.style.height = 'auto'
		target.style.height = target.scrollHeight + 'px'
	}

	function handleDelete(event: Event) {
		event.preventDefault()
		event.stopPropagation()
		// Call the delete callback
		data.onDelete?.()
	}

	// Auto-resize textarea when text changes
	$effect(() => {
		if (textareaElement) {
			textareaElement.style.height = 'auto'
			textareaElement.style.height = textareaElement.scrollHeight + 'px'
		}
	})
	$inspect('dbg selected', selected)
</script>

<div
	class={twMerge(
		'relative w-full h-full rounded-md border group',
		selected ? `outline outline-1 outline-lime-300` : ''
	)}
	style:background-color={data.color}
	onpointerup={() => {
		dragging = false
	}}
	ondragstart={() => {
		dragging = true
	}}
	ondragend={() => {
		dragging = false
	}}
	role="note"
>
	<!-- Delete button -->
	<button
		class="opacity-0 group-hover:opacity-100 hover:opacity-100 absolute -top-6 right-0 w-5 h-5 bg-transparent text-secondary hover:bg-red-500 hover:border-red-500 hover:text-white border rounded-md flex items-center justify-center transition-colors z-10"
		onclick={handleDelete}
		title="Delete note"
		aria-label="Delete note"
	>
		<X size="12" />
	</button>

	<!-- Note content -->
	<div class="p-2 h-full">
		<textarea
			bind:this={textareaElement}
			class="windmillapp w-full h-full !bg-transparent !border-none outline-none shadow-none focus:outline-none focus:ring-transparent resize-none text-xs font-mono overflow-hidden"
			placeholder="Add your note here..."
			value={data.text}
			oninput={handleTextChange}
			spellcheck="false"
		></textarea>
	</div>

	<!-- Node resizer - only visible when selected -->
	<NodeResizer
		isVisible={selected && !dragging}
		minWidth={200}
		minHeight={100}
		lineClass="!border-4 !border-transparent !rounded-md"
		handleClass="!bg-transparent !w-4 !h-4 !border-none !rounded-md"
		onResizeEnd={(_, params) => {
			// Update note size when resizing ends
			if (data.onSizeChange && params.width !== undefined && params.height !== undefined) {
				data.onSizeChange({ width: params.width, height: params.height })
			}
		}}
	/>
</div>

<style>
	textarea::placeholder {
		color: #6b7280;
		opacity: 0.7;
	}

	/* Remove default textarea styling */
	textarea {
		font-family: inherit;
		line-height: 1.4;
	}
</style>
