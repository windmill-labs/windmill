<script lang="ts">
	import autosize from '$lib/autosize'
	import { tick } from 'svelte'
	import type { ContextElement } from './context'
	import AvailableContextList from './AvailableContextList.svelte'
	import { aiChatManager } from './AIChatManager.svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		isFirstMessage: boolean
		disabled: boolean
		onUpdateInstructions: (value: string) => void
		onSendRequest: () => void
		onAddContext: (contextElement: ContextElement) => void
	}

	const {
		availableContext,
		selectedContext,
		isFirstMessage,
		disabled,
		onUpdateInstructions,
		onSendRequest,
		onAddContext
	}: Props = $props()

	let showContextTooltip = $state(false)
	let contextTooltipWord = $state('')
	let tooltipPosition = $state({ x: 0, y: 0 })
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)
	let tooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let selectedSuggestionIndex = $state(0)

	// Properties to copy for caret position calculation
	const properties = [
		'direction',
		'boxSizing',
		'width',
		'height',
		'overflowX',
		'overflowY',
		'borderTopWidth',
		'borderRightWidth',
		'borderBottomWidth',
		'borderLeftWidth',
		'borderStyle',
		'paddingTop',
		'paddingRight',
		'paddingBottom',
		'paddingLeft',
		'fontStyle',
		'fontVariant',
		'fontWeight',
		'fontStretch',
		'fontSize',
		'fontSizeAdjust',
		'lineHeight',
		'fontFamily',
		'textAlign',
		'textTransform',
		'textIndent',
		'textDecoration',
		'letterSpacing',
		'wordSpacing',
		'tabSize',
		'MozTabSize'
	]

	function getCaretCoordinates(element: HTMLTextAreaElement, position: number) {
		// Create mirror div
		const div = document.createElement('div')
		div.id = 'input-textarea-caret-position-mirror-div'
		document.body.appendChild(div)

		// Set styles
		const style = div.style
		const computed = window.getComputedStyle(element)
		const isInput = element.nodeName === 'INPUT'

		// Default textarea styles
		style.whiteSpace = 'pre-wrap'
		if (!isInput) style.wordWrap = 'break-word'

		// Position off-screen
		style.position = 'absolute'
		style.visibility = 'hidden'

		// Transfer properties
		properties.forEach(function (prop) {
			if (isInput && prop === 'lineHeight') {
				// Special case for inputs
				if (computed.boxSizing === 'border-box') {
					const height = parseInt(computed.height)
					const outerHeight =
						parseInt(computed.paddingTop) +
						parseInt(computed.paddingBottom) +
						parseInt(computed.borderTopWidth) +
						parseInt(computed.borderBottomWidth)
					const targetHeight = outerHeight + parseInt(computed.lineHeight)
					if (height > targetHeight) {
						style.lineHeight = height - outerHeight + 'px'
					} else if (height === targetHeight) {
						style.lineHeight = computed.lineHeight
					} else {
						style.lineHeight = '0'
					}
				} else {
					style.lineHeight = computed.height
				}
			} else {
				style[prop] = computed[prop]
			}
		})

		// Firefox special handling
		const isFirefox =
			(window as typeof window & { mozInnerScreenX: number }).mozInnerScreenX != null
		if (isFirefox) {
			if (element.scrollHeight > parseInt(computed.height)) style.overflowY = 'scroll'
		} else {
			style.overflow = 'hidden'
		}

		// Add content before caret
		div.textContent = element.value.substring(0, position)

		// Replace spaces with non-breaking spaces for input elements
		if (isInput) div.textContent = div.textContent.replace(/\s/g, '\u00a0')

		// Create span for position calculation
		const span = document.createElement('span')
		span.textContent = element.value.substring(position) || '.'
		div.appendChild(span)

		// Get coordinates
		const coordinates = {
			top: span.offsetTop + parseInt(computed['borderTopWidth']),
			left: span.offsetLeft + parseInt(computed['borderLeftWidth']),
			height: parseInt(computed['lineHeight'])
		}

		// Cleanup
		document.body.removeChild(div)

		return coordinates
	}

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
		onAddContext(contextElement)
	}

	function updateInstructionsWithContext(contextElement: ContextElement) {
		const index = aiChatManager.instructions.lastIndexOf('@')
		if (index !== -1) {
			const newInstructions =
				aiChatManager.instructions.substring(0, index) + `@${contextElement.title}`
			onUpdateInstructions(newInstructions)
		}
	}

	function handleContextSelection(contextElement: ContextElement) {
		addContextToSelection(contextElement)
		updateInstructionsWithContext(contextElement)
		showContextTooltip = false
	}

	async function updateTooltipPosition(
		availableContext: ContextElement[],
		showContextTooltip: boolean,
		contextTooltipWord: string
	) {
		if (!textarea || !showContextTooltip) return

		try {
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

			// Initial position calculation
			let finalX = rect.left + coords.left - 70
			let finalY: number

			if (isFirstMessage) {
				// Position below the caret line
				finalY = rect.top + coords.top + coords.height - 3
			} else {
				// Position above the caret line
				finalY = rect.top + coords.top - estimatedTooltipHeight - margin
			}

			// Set initial position
			tooltipPosition = {
				x: finalX,
				y: finalY
			}

			// Wait for tooltip to render with initial position
			await tick()

			// Get actual tooltip width if tooltip is rendered
			if (tooltipElement) {
				const tooltipRect = tooltipElement.getBoundingClientRect()
				const tooltipWidth = tooltipRect.width

				// Adjust position if tooltip would overflow right edge
				if (finalX + tooltipWidth > window.innerWidth) {
					finalX = Math.max(10, window.innerWidth - tooltipWidth - 10)

					// Update position after measuring actual width
					tooltipPosition = {
						x: finalX,
						y: finalY
					}
				}
			}
		} catch (error) {
			// Hide tooltip on any error related to position calculation
			console.error('Error updating tooltip position', error)
			showContextTooltip = false
		}
	}

	function handleInput(e: Event) {
		textarea = e.target as HTMLTextAreaElement
		const words = aiChatManager.instructions.split(/\s+/)
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
		onUpdateInstructions(aiChatManager.instructions)
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
					if (
						isInSelectedContext &&
						aiChatManager.instructions.split(' ').pop() === '@' + contextElement.title
					) {
						onSendRequest()
						return
					}
					handleContextSelection(contextElement)
				} else if (contextTooltipWord === '@' && availableContext.length > 0) {
					handleContextSelection(availableContext[0])
				}
			} else {
				onSendRequest()
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

	$effect(() => {
		updateTooltipPosition(availableContext, showContextTooltip, contextTooltipWord)
	})

	export function focus() {
		textarea?.focus()
	}
</script>

<div class="relative w-full px-2 scroll-pb-2">
	<div class="textarea-input absolute top-0 left-0 pointer-events-none">
		<span class="break-words">
			{@html getHighlightedText(aiChatManager.instructions)}
		</span>
	</div>
	<textarea
		bind:this={textarea}
		onkeypress={handleKeyPress}
		onkeydown={handleKeyDown}
		bind:value={aiChatManager.instructions}
		use:autosize
		rows={3}
		oninput={handleInput}
		onblur={() => {
			setTimeout(() => {
				showContextTooltip = false
			}, 200)
		}}
		placeholder={isFirstMessage ? 'Ask anything' : 'Ask followup'}
		class="textarea-input resize-none bg-transparent caret-black dark:caret-white"
		style={aiChatManager.instructions.length > 0
			? 'color: transparent; -webkit-text-fill-color: transparent;'
			: ''}
		{disabled}
	></textarea>
</div>

{#if showContextTooltip}
	<div
		bind:this={tooltipElement}
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

<style>
	.textarea-input {
		padding: 0.25rem 1rem;
		border: 1px solid transparent;
		font-family: inherit;
		font-size: 0.875rem;
		line-height: 1.72;
		white-space: pre-wrap;
		word-break: break-words;
		width: 100%;
		min-height: 3rem;
	}
</style>
