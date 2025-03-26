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

	function updateTooltipPosition(availableContext: ContextElement[], selectedContext: ContextElement[], showContextTooltip: boolean) {
		if (!textarea || !showContextTooltip) return
		
		const coords = getCaretCoordinates(textarea, textarea.selectionEnd)
		const rect = textarea.getBoundingClientRect()

		const availableContextCount = availableContext.length - selectedContext.length
		const offset = (isFirstMessage ? 20 : -(55 + 30 * (availableContextCount - 1)))

		tooltipPosition = {
			x: rect.left + coords.left - 70,
			y: rect.top + coords.top + offset
		}
	}

	function handleInput(e: Event) {
		textarea = e.target as HTMLTextAreaElement
		const words = instructions.split(/\s+/)
		const lastWord = words[words.length - 1]
		
		if (lastWord.startsWith('@') && (!availableContext.find((c) => c.title === lastWord.slice(1)) || !selectedContext.find((c) => c.title === lastWord.slice(1)))) {
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

	$: updateTooltipPosition(availableContext, selectedContext, showContextTooltip)

</script>

<div class="relative w-full px-2 scroll-pb-2">
	<div
		class="absolute top-0 left-0 w-full h-full min-h-12 px-4 text-sm pt-1 pointer-events-none"
		style="line-height: 1.72"
	>
		<span class="break-words">
			{@html getHighlightedText(instructions)}
		</span>
	</div>
	<textarea
		bind:this={textarea}
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
		placeholder={isFirstMessage ? 'Ask anything' : 'Ask followup'}
		class="resize-none bg-transparent caret-white"
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
			showAllAvailable={true}
			stringSearch={contextTooltipWord.slice(1)}
		/>
	</div>
{/if} 