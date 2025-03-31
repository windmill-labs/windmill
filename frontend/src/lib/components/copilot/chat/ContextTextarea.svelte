<script lang="ts">
	import autosize from '$lib/autosize'
	import { createEventDispatcher } from 'svelte'
	import type { ContextElement } from './core'
	import AvailableContextList from './AvailableContextList.svelte'
	import getCaretCoordinates from 'textarea-caret'

	export let instructions: string
	export let availableContext: ContextElement[]
	export let selectedContext: ContextElement[]
	export let isFirstMessage: boolean

	const dispatch = createEventDispatcher<{
		updateInstructions: { value: string }
		sendRequest: null
		addContext: { contextElement: ContextElement }
	}>()

	let showContextTooltip = false
	let contextTooltipWord = ''
	let tooltipPosition = { x: 0, y: 0 }
	let textarea: HTMLTextAreaElement
	let selectedSuggestionIndex = 0

	function getHighlightedText(text: string) {
		return text.replace(/@[\w/.-]+/g, (match) => {
			const contextElement = availableContext.find((c) => c.title === match.slice(1))
			if (contextElement) {
				return `<span class="bg-black dark:bg-white text-white dark:text-black z-10">${match}</span>`
			}
			return match
		})
	}

	function addContextToSelection(contextElement: ContextElement) {
		dispatch('addContext', { contextElement })
	}

	function updateInstructionsWithContext(contextElement: ContextElement) {
		const index = instructions.lastIndexOf('@')
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

	function updateTooltipPosition(
		availableContext: ContextElement[],
		showContextTooltip: boolean,
		contextTooltipWord: string
	) {
		if (!textarea || !showContextTooltip) return

		const coords = getCaretCoordinates(textarea, textarea.selectionEnd)
		const rect = textarea.getBoundingClientRect()

		const filteredAvailableContext = availableContext.filter(
			(c) => !contextTooltipWord || c.title.toLowerCase().includes(contextTooltipWord.slice(1))
		)

		const itemHeight = 28 // Estimated height of one item + gap (Button: p-1(8px) + text-xs(16px) = 24px; Parent: gap-1(4px) = 28px)
		const containerPadding = 8 // p-1 top + p-1 bottom = 4px + 4px = 8px
		const maxHeight = 192 + containerPadding // max-h-48 (192px) + containerPadding (8px)

		// Calculate uncapped height, subtract gap from last item as it's not needed
		const numItems = filteredAvailableContext.length
		let uncappedHeight =
			numItems > 0 ? numItems * itemHeight - 4 + containerPadding : containerPadding
		// Ensure height is at least containerPadding even if no items
		uncappedHeight = Math.max(uncappedHeight, containerPadding)

		const estimatedTooltipHeight = Math.min(uncappedHeight, maxHeight)
		const margin = 6 // Small margin between caret and tooltip

		let finalY: number

		if (isFirstMessage) {
			// Position below the caret line
			finalY = rect.top + coords.top + coords.height - 3
		} else {
			// Position above the caret line
			finalY = rect.top + coords.top - estimatedTooltipHeight - margin
		}

		tooltipPosition = {
			x: rect.left + coords.left - 70,
			y: finalY
		}
	}

	function handleInput(e: Event) {
		textarea = e.target as HTMLTextAreaElement
		const words = instructions.split(/\s+/)
		const lastWord = words[words.length - 1]

		if (
			lastWord.startsWith('@') &&
			(!availableContext.find((c) => c.title === lastWord.slice(1)) ||
				!selectedContext.find((c) => c.title === lastWord.slice(1)))
		) {
			showContextTooltip = true
			contextTooltipWord = lastWord
		} else {
			showContextTooltip = false
			contextTooltipWord = ''
			selectedSuggestionIndex = 0
		}
		dispatch('updateInstructions', { value: instructions })
	}

	function handleKeyPress(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			if (contextTooltipWord) {
				const filteredContext = availableContext.filter(
					(c) => !contextTooltipWord || c.title.toLowerCase().includes(contextTooltipWord.slice(1))
				)
				const contextElement = filteredContext[selectedSuggestionIndex]
				if (contextElement) {
					const isInSelectedContext = selectedContext.find(
						(c) => c.title === contextElement.title && c.type === contextElement.type
					)
					// If the context element is already in the selected context and the last word in the instructions is the same as the context element title, send request
					if (isInSelectedContext && instructions.split(' ').pop() === '@' + contextElement.title) {
						dispatch('sendRequest')
						return
					}
					handleContextSelection(contextElement)
				} else if (contextTooltipWord === '@' && availableContext.length > 0) {
					handleContextSelection(availableContext[0])
				}
			} else {
				dispatch('sendRequest')
			}
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!showContextTooltip) return

		const filteredContext = availableContext.filter(
			(c) => !contextTooltipWord || c.title.toLowerCase().includes(contextTooltipWord.slice(1))
		)

		if (e.key === 'Tab') {
			e.preventDefault()
			const contextElement = filteredContext[selectedSuggestionIndex]
			if (contextElement) {
				handleContextSelection(contextElement)
			}
		}

		if (e.key === 'ArrowDown') {
			e.preventDefault()
			selectedSuggestionIndex = (selectedSuggestionIndex + 1) % filteredContext.length
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			selectedSuggestionIndex =
				(selectedSuggestionIndex - 1 + filteredContext.length) % filteredContext.length
		}
	}

	$: updateTooltipPosition(availableContext, showContextTooltip, contextTooltipWord)
</script>

<div class="relative w-full px-2 scroll-pb-2">
	<div
		class="absolute top-0 left-0 w-full h-full min-h-12 px-4 text-sm pt-1 pointer-events-none"
		style="line-height: 1.72"
	>
		<span class="break-words" style="white-space: pre-wrap;">
			{@html getHighlightedText(instructions)}
		</span>
	</div>
	<textarea
		bind:this={textarea}
		on:keypress={handleKeyPress}
		on:keydown={handleKeyDown}
		bind:value={instructions}
		use:autosize
		rows={3}
		on:input={handleInput}
		on:blur={() => {
			setTimeout(() => {
				showContextTooltip = false
			}, 100)
		}}
		placeholder={isFirstMessage ? 'Ask anything' : 'Ask followup'}
		class="resize-none bg-transparent caret-black dark:caret-white"
		style={instructions.length > 0
			? 'color: transparent; -webkit-text-fill-color: transparent;'
			: ''}
	/>
</div>

{#if showContextTooltip}
	<div
		class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg z-50"
		style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;"
	>
		<AvailableContextList
			{availableContext}
			{selectedContext}
			onSelect={(element) => {
				handleContextSelection(element)
			}}
			showAllAvailable={true}
			stringSearch={contextTooltipWord.slice(1)}
			selectedIndex={selectedSuggestionIndex}
		/>
	</div>
{/if}
