<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AvailableContextList from './AvailableContextList.svelte'
	import AppAvailableContextList from './AppAvailableContextList.svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import ContextTextarea from './ContextTextarea.svelte'
	import autosize from '$lib/autosize'
	import type { ContextElement } from './context'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { Snippet } from 'svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { tick } from 'svelte'

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		isFirstMessage?: boolean
		disabled?: boolean
		placeholder?: string
		initialInstructions?: string
		editingMessageIndex?: number | null
		onEditEnd?: () => void
		className?: string
		onClickOutside?: () => void
		onSendRequest?: (instructions: string) => void
		showContext?: boolean
		bottomRightSnippet?: Snippet
		onKeyDown?: (e: KeyboardEvent) => void
	}

	let {
		availableContext,
		selectedContext = $bindable([]),
		disabled = false,
		isFirstMessage = false,
		placeholder,
		initialInstructions = '',
		editingMessageIndex = null,
		onEditEnd = () => {},
		className = '',
		onClickOutside = () => {},
		onSendRequest = undefined,
		showContext = true,
		bottomRightSnippet,
		onKeyDown = undefined
	}: Props = $props()

	// Generate mode-specific placeholder
	const modePlaceholder = $derived.by(() => {
		if (!isFirstMessage) {
			return 'Ask followup'
		}

		if (placeholder) {
			return placeholder
		}

		switch (aiChatManager.mode) {
			case AIMode.SCRIPT:
				return 'Modify this script...'
			case AIMode.FLOW:
				return 'Modify this flow...'
			case AIMode.APP:
				return 'Modify this app...'
			case AIMode.NAVIGATOR:
				return 'Navigate Windmill UI...'
			case AIMode.API:
				return 'Make API calls...'
			case AIMode.ASK:
				return 'Ask questions about Windmill...'
			default:
				return 'Ask anything'
		}
	})

	let contextTextareaComponent: ContextTextarea | undefined = $state()
	let instructionsTextareaComponent: HTMLTextAreaElement | undefined = $state()
	let instructions = $state(initialInstructions)

	// App mode @ mention state
	let showAppContextTooltip = $state(false)
	let appContextTooltipWord = $state('')
	let appTooltipPosition = $state({ x: 0, y: 0 })
	let appTooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let appTooltipCurrentViewNumber = $state(0)

	export function focusInput() {
		if (aiChatManager.mode === AIMode.SCRIPT || aiChatManager.mode === AIMode.FLOW) {
			contextTextareaComponent?.focus()
		} else {
			instructionsTextareaComponent?.focus()
		}
	}

	function clickOutside(node: HTMLElement) {
		function handleClick(event: MouseEvent) {
			if (node && !node.contains(event.target as Node)) {
				onClickOutside()
			}
		}

		document.addEventListener('click', handleClick, true)
		return {
			destroy() {
				document.removeEventListener('click', handleClick, true)
			}
		}
	}

	function addContextToSelection(contextElement: ContextElement) {
		if (!selectedContext || !availableContext) return

		const alreadySelected = selectedContext.find(
			(c) => c.type === contextElement.type && c.title === contextElement.title
		)
		if (alreadySelected) return

		// Workspace items are fetched on-demand and not in availableContext,
		// so skip the availableContext check for them
		const isWorkspaceItem =
			contextElement.type === 'workspace_script' || contextElement.type === 'workspace_flow'
		if (
			!isWorkspaceItem &&
			!availableContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			)
		) {
			return
		}

		selectedContext = [...selectedContext, contextElement]

		// If it's a datatable table, add it to the app's whitelisted tables
		if (
			contextElement.type === 'app_datatable' &&
			aiChatManager.mode === AIMode.APP &&
			aiChatManager.appAiChatHelpers
		) {
			aiChatManager.appAiChatHelpers.addTableToWhitelist(
				contextElement.datatableName,
				contextElement.schemaName,
				contextElement.tableName
			)
		}
	}

	function sendRequest() {
		if (aiChatManager.loading) {
			return
		}
		if (editingMessageIndex !== null) {
			aiChatManager.restartGeneration(editingMessageIndex, instructions)
			onEditEnd()
		} else {
			aiChatManager.sendRequest({ instructions })
			instructions = ''
		}
	}

	$effect(() => {
		if (editingMessageIndex !== null) {
			focusInput()
		}
	})

	// Properties to copy for caret position calculation (app mode)
	const caretProperties = [
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
		const div = document.createElement('div')
		div.id = 'input-textarea-caret-position-mirror-div'
		document.body.appendChild(div)

		const style = div.style
		const computed = window.getComputedStyle(element)
		const isInput = element.nodeName === 'INPUT'

		style.whiteSpace = 'pre-wrap'
		if (!isInput) style.wordWrap = 'break-word'

		style.position = 'absolute'
		style.visibility = 'hidden'

		caretProperties.forEach(function (prop) {
			if (isInput && prop === 'lineHeight') {
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

		const isFirefox =
			(window as typeof window & { mozInnerScreenX: number }).mozInnerScreenX != null
		if (isFirefox) {
			if (element.scrollHeight > parseInt(computed.height)) style.overflowY = 'scroll'
		} else {
			style.overflow = 'hidden'
		}

		div.textContent = element.value.substring(0, position)

		if (isInput) div.textContent = div.textContent.replace(/\s/g, '\u00a0')

		const span = document.createElement('span')
		span.textContent = element.value.substring(position) || '.'
		div.appendChild(span)

		const coordinates = {
			top: span.offsetTop + parseInt(computed['borderTopWidth']),
			left: span.offsetLeft + parseInt(computed['borderLeftWidth']),
			height: parseInt(computed['lineHeight'])
		}

		document.body.removeChild(div)

		return coordinates
	}

	async function updateAppTooltipPosition(currentViewItemsNumber: number) {
		if (!instructionsTextareaComponent) return

		try {
			const coords = getCaretCoordinates(
				instructionsTextareaComponent,
				instructionsTextareaComponent.selectionEnd
			)
			const rect = instructionsTextareaComponent.getBoundingClientRect()

			const itemHeight = 28
			const containerPadding = 8
			const maxHeight = 192 + containerPadding

			const numItems = currentViewItemsNumber
			let uncappedHeight =
				numItems > 0 ? numItems * itemHeight - 4 + containerPadding : containerPadding
			uncappedHeight = Math.max(uncappedHeight, containerPadding)

			const estimatedTooltipHeight = Math.min(uncappedHeight, maxHeight)
			const margin = 6

			let finalX = rect.left + coords.left - 70
			let finalY: number

			if (isFirstMessage) {
				finalY = rect.top + coords.top + coords.height - 3
			} else {
				finalY = rect.top + coords.top - estimatedTooltipHeight - margin
			}

			appTooltipPosition = {
				x: finalX,
				y: finalY
			}

			await tick()

			if (appTooltipElement) {
				const tooltipRect = appTooltipElement.getBoundingClientRect()
				const tooltipWidth = tooltipRect.width

				if (finalX + tooltipWidth > window.innerWidth) {
					finalX = Math.max(10, window.innerWidth - tooltipWidth - 10)

					appTooltipPosition = {
						x: finalX,
						y: finalY
					}
				}
			}
		} catch (error) {
			console.error('Error updating tooltip position', error)
			showAppContextTooltip = false
		}
	}

	function handleAppInput(_e: Event) {
		const words = instructions.split(/\s+/)
		const lastWord = words[words.length - 1]

		if (
			lastWord.startsWith('@') &&
			(!availableContext.find((c) => c.title === lastWord.slice(1)) ||
				!selectedContext.find((c) => c.title === lastWord.slice(1)))
		) {
			showAppContextTooltip = true
			appContextTooltipWord = lastWord
		} else {
			showAppContextTooltip = false
			appContextTooltipWord = ''
		}
	}

	function handleAppContextSelection(contextElement: ContextElement) {
		addContextToSelection(contextElement)
		// Update instructions with the selected context title
		const index = instructions.lastIndexOf('@')
		if (index !== -1) {
			instructions = instructions.substring(0, index) + `@${contextElement.title}`
		}
		showAppContextTooltip = false
	}

	$effect(() => {
		if (showAppContextTooltip) {
			updateAppTooltipPosition(appTooltipCurrentViewNumber)
		}
	})
</script>

<div use:clickOutside class="relative">
	{#if aiChatManager.mode === AIMode.SCRIPT || aiChatManager.mode === AIMode.FLOW}
		{#if showContext}
			<div class="flex flex-row gap-1 mb-1 overflow-scroll pt-2 no-scrollbar">
				<Popover>
					<svelte:fragment slot="trigger">
						<div
							class="border rounded-md px-1 py-0.5 font-normal text-primary text-xs hover:bg-surface-hover bg-surface"
							>@</div
						>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<AvailableContextList
							{availableContext}
							{selectedContext}
							onSelect={(element) => {
								addContextToSelection(element)
								close()
							}}
							onSelectWorkspaceItem={(element) => {
								addContextToSelection(element)
								close()
							}}
						/>
					</svelte:fragment>
				</Popover>
				{#each selectedContext as element}
					<ContextElementBadge
						contextElement={element}
						deletable
						onDelete={() => {
							selectedContext = selectedContext?.filter(
								(c) => c.type !== element.type || c.title !== element.title
							)
						}}
					/>
				{/each}
			</div>
		{/if}
		<ContextTextarea
			bind:this={contextTextareaComponent}
			bind:value={instructions}
			{availableContext}
			{selectedContext}
			{isFirstMessage}
			placeholder={modePlaceholder}
			onAddContext={(contextElement) => addContextToSelection(contextElement)}
			onSendRequest={() => {
				if (disabled) {
					return
				}
				onSendRequest ? onSendRequest(instructions) : sendRequest()
			}}
			{disabled}
			{onKeyDown}
		/>
	{:else if aiChatManager.mode === AIMode.APP}
		{#if showContext}
			<div class="flex flex-row gap-1 mb-1 overflow-scroll pt-2 no-scrollbar">
				<Popover>
					<svelte:fragment slot="trigger">
						<div
							class="border rounded-md px-1 py-0.5 font-normal text-primary text-xs hover:bg-surface-hover bg-surface"
							>@</div
						>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<AppAvailableContextList
							{availableContext}
							{selectedContext}
							onSelect={(element) => {
								addContextToSelection(element)
								close()
							}}
						/>
					</svelte:fragment>
				</Popover>
				{#each selectedContext as element (element.type + '-' + element.title)}
					<ContextElementBadge
						contextElement={element}
						deletable
						onDelete={() => {
							selectedContext = selectedContext?.filter(
								(c) => c.type !== element.type || c.title !== element.title
							)
						}}
					/>
				{/each}
			</div>
		{/if}
		<div class={twMerge('relative w-full scroll-pb-2', className)}>
			<textarea
				bind:this={instructionsTextareaComponent}
				bind:value={instructions}
				use:autosize
				oninput={handleAppInput}
				onblur={() => {
					setTimeout(() => {
						showAppContextTooltip = false
					}, 200)
				}}
				onkeydown={(e) => {
					if (onKeyDown) {
						onKeyDown(e)
					}
					if (showAppContextTooltip) {
						// avoid new line after Enter in the tooltip
						if (e.key === 'Enter') {
							e.preventDefault()
						}
						return
					}
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					}
				}}
				rows={3}
				placeholder={modePlaceholder}
				class="resize-none"
				{disabled}
			></textarea>
		</div>
		{#if showAppContextTooltip}
			<Portal target="body">
				<div
					bind:this={appTooltipElement}
					class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg"
					style="left: {appTooltipPosition.x}px; top: {appTooltipPosition.y}px; z-index: {zIndexes.tooltip};"
				>
					<AppAvailableContextList
						{availableContext}
						{selectedContext}
						onSelect={(element) => {
							handleAppContextSelection(element)
						}}
						showAllAvailable={true}
						stringSearch={appContextTooltipWord.slice(1)}
						onViewChange={(newNumber) => {
							appTooltipCurrentViewNumber = newNumber
						}}
						setShowing={(showing) => {
							showAppContextTooltip = showing
						}}
					/>
				</div>
			</Portal>
		{/if}
	{:else}
		<div class={twMerge('relative w-full scroll-pb-2 pt-2', className)}>
			<textarea
				bind:this={instructionsTextareaComponent}
				bind:value={instructions}
				use:autosize
				onkeydown={(e) => {
					if (onKeyDown) {
						onKeyDown(e)
					}
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					}
				}}
				rows={3}
				placeholder={modePlaceholder}
				class="resize-none"
				{disabled}
			></textarea>
		</div>
	{/if}
	{#if bottomRightSnippet}
		<div class="absolute bottom-2 right-2">
			{@render bottomRightSnippet()}
		</div>
	{/if}
</div>
