<script lang="ts">
	import autosize from '$lib/autosize'
	import { createEventDispatcher } from 'svelte'
	import type { ContextElement, SelectedContext } from './core'
	import AvailableContextList from './AvailableContextList.svelte'
	import getCaretCoordinates from 'textarea-caret'

	export let instructions: string
	export let availableContext: ContextElement[]
	export let selectedContext: SelectedContext[]
	export let placeholder: string

	const dispatch = createEventDispatcher<{
		updateInstructions: { value: string }
		sendRequest: null
		addContext: { contextElement: ContextElement }
	}>()

	let showContextTooltip = false
	let contextTooltipWord = ''
	let tooltipPosition = { x: 0, y: 0 }

	function getHighlightedText(text: string) {
		return text.replace(/@[\w/.-]+/g, (match) => {
			const contextElement = availableContext.find((c) => c.title === match.slice(1))
			if (contextElement) {
				return `<span class="bg-white text-black z-10">${match}</span>`
			}
			return match
		})
	}

	function addContextToSelection(contextElement: ContextElement) {
		dispatch('addContext', { contextElement })
	}

	function updateInstructionsWithContext(contextElement: ContextElement) {
		const index = instructions.lastIndexOf("@")
		if (index !== -1) {
			const newInstructions = instructions.substring(0, index) + `@${contextElement.title}`
			dispatch('updateInstructions', { value: newInstructions })
		}
	}

	function handleContextSelection(contextElement: ContextElement) {
		addContextToSelection(contextElement)
		updateInstructionsWithContext(contextElement)
		showContextTooltip = false
	}

	function handleInput(e: Event) {
		const textarea = e.target as HTMLTextAreaElement
		const words = instructions.split(/\s+/)
		const lastWord = words[words.length - 1]
		
        // If the last word is a context and it's not in the available context or selected context, show the tooltip
		if (lastWord.startsWith('@') && (!availableContext.find((c) => c.title === lastWord.slice(1)) || !selectedContext.find((c) => c.title === lastWord.slice(1)))) {
			const coords = getCaretCoordinates(textarea, textarea.selectionEnd)
			const rect = textarea.getBoundingClientRect()

			tooltipPosition = {
				x: rect.left + coords.left - 70,
				y: rect.top + coords.top + 20
			}

			showContextTooltip = true
			contextTooltipWord = lastWord
		} else {
			showContextTooltip = false
			contextTooltipWord = ''
		}
        dispatch('updateInstructions', { value: instructions })
	}

	function handleKeyPress(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			if (contextTooltipWord) {
				const contextElement = availableContext.find((c) => c.title.includes(contextTooltipWord.slice(1)))
				if (contextElement) {
					handleContextSelection(contextElement)
				} else if (contextTooltipWord === '@' && availableContext.length > 0) {
					handleContextSelection(availableContext[0])
				}
			} else {
				dispatch('sendRequest')
			}
		}
	}
</script>

<div class="relative w-full">
	<div
		class="absolute top-0 left-0 w-full h-full min-h-12 p-2 text-sm pt-1"
		style="line-height: 1.72"
	>
		<span class="break-words">
			{@html getHighlightedText(instructions)}
		</span>
	</div>
	<textarea
		on:keypress={handleKeyPress}
		bind:value={instructions}
		use:autosize
		rows={3}
		on:input={handleInput}
		on:blur={() => {
			setTimeout(() => {
				showContextTooltip = false
			}, 100)
		}}
		{placeholder}
		class="resize-none bg-transparent absolute top-0 left-0 w-full h-full caret-white"
		style="{instructions.length > 0 ? 'color: transparent; -webkit-text-fill-color: transparent;' : ''}"
	/>
</div>

{#if showContextTooltip}
	<div
		class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg p-2 z-50"
		style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;"
	>
		<AvailableContextList
			{availableContext}
			{selectedContext}
			onSelect={(element) => {
				handleContextSelection(element)
			}}
		/>
	</div>
{/if} 