<script lang="ts">
	import autosize from '$lib/autosize'
	import { tick } from 'svelte'
	import type { ContextElement } from './context'
	import AvailableContextList from './AvailableContextList.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		value: string
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		isFirstMessage: boolean
		placeholder: string
		disabled: boolean
		onSendRequest: () => void
		onAddContext: (contextElement: ContextElement) => void
		className?: string
		onKeyDown?: (e: KeyboardEvent) => void
	}

	let {
		value = $bindable(''),
		availableContext,
		selectedContext,
		isFirstMessage,
		placeholder,
		disabled,
		onSendRequest,
		onAddContext,
		className = '',
		onKeyDown = undefined
	}: Props = $props()

	let showContextTooltip = $state(false)
	let contextTooltipWord = $state('')
	let tooltipPosition = $state({ x: 0, y: 0 })
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)
	let tooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let tooltipCurrentViewNumber = $state(0)

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
		return text.replace(/@[\w/.\-\[\]]+/g, (match) => {
			const title = match.slice(1)
			const inContext =
				availableContext.find((c) => c.title === title) ||
				selectedContext.find((c) => c.title === title)
			if (inContext) {
				return `<span class="bg-black dark:bg-white text-white dark:text-black z-10">${match}</span>`
			}
			return match
		})
	}

	function addContextToSelection(contextElement: ContextElement) {
		onAddContext(contextElement)
	}

	function updateInstructionsWithContext(contextElement: ContextElement) {
		const index = value.lastIndexOf('@')
		if (index !== -1) {
			const newInstructions = value.substring(0, index) + `@${contextElement.title}`
			value = newInstructions
		}
	}

	function handleContextSelection(contextElement: ContextElement) {
		addContextToSelection(contextElement)
		updateInstructionsWithContext(contextElement)
		showContextTooltip = false
	}

	async function updateTooltipPosition(currentViewItemsNumber: number) {
		if (!textarea) return

		try {
			const coords = getCaretCoordinates(textarea, textarea.selectionEnd)
			const rect = textarea.getBoundingClientRect()

			const itemHeight = 28 // Estimated height of one item + gap (Button: p-1(8px) + text-xs(16px) = 24px; Parent: gap-1(4px) = 28px)
			const containerPadding = 8 // p-1 top + p-1 bottom = 4px + 4px = 8px
			const maxHeight = 192 + containerPadding // max-h-48 (192px) + containerPadding (8px)

			// Calculate uncapped height, subtract gap from last item as it's not needed
			const numItems = currentViewItemsNumber
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
		const words = value.split(/\s+/)
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
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		// Pass to parent first if provided
		if (onKeyDown) {
			onKeyDown(e)
		}

		if (showContextTooltip) {
			// avoid new line after Enter in the tooltip
			if (e.key === 'Enter') {
				e.preventDefault()
			}
			return
		}

		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			onSendRequest()
		}
	}

	$effect(() => {
		if (showContextTooltip) {
			updateTooltipPosition(tooltipCurrentViewNumber)
		}
	})

	export function focus() {
		textarea?.focus()
	}
</script>

<div class="relative w-full scroll-pb-2 bg-surface">
	<div
		class={twMerge(
			'textarea-input absolute top-0 left-0 pointer-events-none py-1 !px-2',
			className
		)}
	>
		<span class="break-words">
			{@html getHighlightedText(value)}
		</span>
	</div>
	<textarea
		bind:this={textarea}
		onkeydown={handleKeyDown}
		bind:value
		use:autosize
		rows={3}
		oninput={handleInput}
		onblur={() => {
			setTimeout(() => {
				// Don't close if focus moved to inside the tooltip (e.g., search input)
				if (tooltipElement?.contains(document.activeElement)) {
					return
				}
				showContextTooltip = false
			}, 200)
		}}
		{placeholder}
		class={twMerge(
			'textarea-input resize-none bg-transparent caret-black dark:caret-white overflow-clip',
			className
		)}
		style={value.length > 0 ? 'color: transparent; -webkit-text-fill-color: transparent;' : ''}
		{disabled}
	></textarea>
</div>

{#if showContextTooltip}
	<Portal target="body">
		<div
			bind:this={tooltipElement}
			class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg"
			style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px; z-index: {zIndexes.tooltip};"
		>
			<AvailableContextList
				{availableContext}
				{selectedContext}
				onSelect={(element) => {
					handleContextSelection(element)
				}}
				onSelectWorkspaceItem={(element) => {
					onAddContext(element)
					updateInstructionsWithContext(element)
					showContextTooltip = false
					// Refocus the textarea since focus may have been on the search input
					setTimeout(() => textarea?.focus(), 0)
				}}
				showAllAvailable={true}
				stringSearch={contextTooltipWord.slice(1)}
				onViewChange={(newNumber) => {
					tooltipCurrentViewNumber = newNumber
				}}
				setShowing={(showing) => {
					showContextTooltip = showing
				}}
			/>
		</div>
	</Portal>
{/if}

<style>
	.textarea-input {
		padding: 0.25rem;
		border: 1px solid transparent;
		font-family: inherit;
		font-size: 0.75rem;
		line-height: 1.72;
		white-space: pre-wrap;
		word-break: break-words;
		width: 100%;
		min-height: 3rem;
	}
</style>
