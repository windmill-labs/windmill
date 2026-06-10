<script lang="ts">
	import autosize from '$lib/autosize'
	import { tick } from 'svelte'
	import type { ContextElement } from './context'
	import ChatContextPicker from './ChatContextPicker.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { twMerge } from 'tailwind-merge'
	import { CHAT_INPUT_PADDING } from './aiChatManagerContext'
	import { createFloatingActions, createVirtualElement } from 'svelte-floating-ui'
	import { flip, offset, shift } from 'svelte-floating-ui/dom'
	import {
		type PasteAttachment,
		makePasteToken,
		nextPasteId,
		pasteTokenRegex,
		shouldCollapsePaste
	} from './pasteTokens'

	interface Props {
		value: string
		pastes?: PasteAttachment[]
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		placeholder: string
		disabled: boolean
		onSendRequest: () => void
		onAddContext: (contextElement: ContextElement) => void
		/** Called when the user deletes a previously-inserted `@title` mention
		 * from the textarea. The host should drop the matching entry from
		 * selectedContext (only items with `deletable !== false` are reported). */
		onRemoveContext?: (contextElement: ContextElement) => void
		className?: string
		onKeyDown?: (e: KeyboardEvent) => void
	}

	let {
		value = $bindable(''),
		pastes = $bindable(),
		availableContext,
		selectedContext,
		placeholder,
		disabled,
		onSendRequest,
		onAddContext,
		onRemoveContext,
		className = '',
		onKeyDown = undefined
	}: Props = $props()

	const MENTION_RE = /@[\w/.\-\[\]]+/g
	function extractMentions(text: string): Set<string> {
		const out = new Set<string>()
		for (const m of text.matchAll(MENTION_RE)) out.add(m[0].slice(1))
		return out
	}

	// Titles currently appearing as `@title` mentions in the textarea. Compared
	// against the previous snapshot in a $effect (NOT inside handleInput —
	// the picker mutates `value` programmatically via `updateInstructionsWithContext`,
	// which doesn't fire `oninput`, so a handleInput-only diff goes stale).
	const mentionedTitles = $derived(extractMentions(value))
	let prevMentionedTitles = $state<Set<string>>(new Set())

	let showContextTooltip = $state(false)
	let contextTooltipWord = $state('')
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)
	let tooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let chatContextPicker: ChatContextPicker | undefined = $state()

	// Virtual reference anchored at the `@` that opened the mention (not the
	// caret), so the picker stays put while the user types the query.
	// svelte-floating-ui's `createVirtualElement` takes a raw ClientRect and
	// wraps it in a function internally — re-`update()` on each anchor move.
	let anchorRect: DOMRect = new DOMRect(0, 0, 1, 16)
	const anchorRef = createVirtualElement({ getBoundingClientRect: anchorRect })

	const [floatingRef, floatingContent, updateFloating] = createFloatingActions({
		strategy: 'fixed',
		placement: 'bottom-start',
		// flip handles above/below only; horizontal overflow is solved by shift
		// (picker slides left to fit) instead of flipping to `bottom-end` which
		// would re-anchor the picker's right edge to the `@`.
		middleware: [offset(6), flip({ crossAxis: false }), shift({ padding: 10 })],
		autoUpdate: true
	})

	// Calling the reference action as a function (instead of `use:floatingRef`
	// on a DOM node): svelte-floating-ui's `referenceAction` detects
	// `'subscribe' in node` and subscribes to the virtual-element store. This
	// is the supported path for virtual references — see the library's
	// `referenceAction` / `setupVirtualElementObserver` in dist/index.js.
	floatingRef(anchorRef)

	// Mirrors the textarea's vertical scroll so the highlight overlay stays
	// aligned once the input is capped (max-height) and scrolls internally.
	let scrollTop = $state(0)

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

	function escapeHtml(text: string) {
		return text
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;')
			.replace(/'/g, '&#39;')
	}

	function getHighlightedText(text: string) {
		let html = escapeHtml(text)
		// Wrap collapsed-paste tokens as clickable chips. The span keeps the exact
		// token text (label + zero-width id) so its width matches the underlying
		// transparent textarea text and the caret stays aligned.
		html = html.replace(pasteTokenRegex(), (match, zw: string) => {
			const att = pastes?.find((p) => p.id === zw.length)
			if (!att) return match
			return `<span data-paste-id="${att.id}" class="rounded bg-surface-secondary text-secondary cursor-pointer pointer-events-auto">${match}</span>`
		})
		html = html.replace(/@[\w/.\-\[\]]+/g, (match) => {
			const title = match.slice(1)
			const inContext =
				availableContext.find((c) => c.title === title) ||
				selectedContext.find((c) => c.title === title)
			if (inContext) {
				return `<span class="bg-surface-accent-selected text-primary rounded box-decoration-clone z-10">${match}</span>`
			}
			return match
		})
		return html
	}

	// On a large paste, register the blob and insert a compact token instead of
	// the raw lines (see pasteTokens.ts). Smaller pastes fall through to default.
	function handlePaste(e: ClipboardEvent) {
		const text = e.clipboardData?.getData('text/plain') ?? ''
		if (!text || !shouldCollapsePaste(text)) return
		e.preventDefault()
		const ta = e.currentTarget as HTMLTextAreaElement
		const start = ta.selectionStart ?? value.length
		const end = ta.selectionEnd ?? value.length
		const att: PasteAttachment = {
			id: nextPasteId(pastes ?? []),
			lines: text.split('\n').length,
			content: text
		}
		const token = makePasteToken(att)
		value = value.slice(0, start) + token + value.slice(end)
		pastes = [...(pastes ?? []), att]
		const caret = start + token.length
		tick().then(() => ta.setSelectionRange(caret, caret))
	}

	// Click on a chip in the input expands it back to its raw lines (one-way).
	function expandPasteInInput(id: number) {
		const att = pastes?.find((p) => p.id === id)
		if (!att) return
		const token = makePasteToken(att)
		const idx = value.indexOf(token)
		if (idx === -1) return
		value = value.slice(0, idx) + att.content + value.slice(idx + token.length)
		pastes = (pastes ?? []).filter((p) => p.id !== id)
		const caret = idx + att.content.length
		tick().then(() => {
			textarea?.focus()
			textarea?.setSelectionRange(caret, caret)
		})
	}

	// Delegated as an action (not an inline onclick) so the pointer-events-none
	// overlay div doesn't trip a11y static-interaction lints; only the chip spans
	// inside set pointer-events: auto, and their clicks bubble here.
	function chipClickDelegate(node: HTMLElement) {
		const handler = (e: Event) => {
			const chip = (e.target as HTMLElement).closest('[data-paste-id]')
			if (!chip) return
			expandPasteInInput(Number(chip.getAttribute('data-paste-id')))
		}
		node.addEventListener('click', handler)
		return {
			destroy() {
				node.removeEventListener('click', handler)
			}
		}
	}

	// Backspace/Delete at a token boundary removes the whole chip + its blob.
	function handlePasteDeletion(e: KeyboardEvent): boolean {
		if ((e.key !== 'Backspace' && e.key !== 'Delete') || !textarea) return false
		const start = textarea.selectionStart
		const end = textarea.selectionEnd
		if (start === null || start !== end) return false
		for (const m of value.matchAll(pasteTokenRegex())) {
			if (m.index === undefined) continue
			const tokenStart = m.index
			const tokenEnd = m.index + m[0].length
			const atEnd = e.key === 'Backspace' && start === tokenEnd
			const atStart = e.key === 'Delete' && start === tokenStart
			if (atEnd || atStart) {
				e.preventDefault()
				value = value.slice(0, tokenStart) + value.slice(tokenEnd)
				pastes = (pastes ?? []).filter((p) => p.id !== m[1].length)
				tick().then(() => textarea?.setSelectionRange(tokenStart, tokenStart))
				return true
			}
		}
		return false
	}

	// Arrow-Left/Right step over a paste chip as one unit, so the caret jumps
	// edge-to-edge instead of crawling through the (invisible) token characters.
	// Also snaps the caret out if it somehow lands inside a token. Shift extends
	// the selection across the whole chip; word/line jumps (alt/cmd/ctrl) are
	// left to the browser.
	function handlePasteCaretSkip(e: KeyboardEvent): boolean {
		if ((e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') || !textarea) return false
		if (e.altKey || e.metaKey || e.ctrlKey) return false
		const right = e.key === 'ArrowRight'
		const collapsed = textarea.selectionStart === textarea.selectionEnd
		// The end the caret is moving; for a non-collapsed selection that's the
		// side opposite the anchor (given by selectionDirection).
		const caret =
			!collapsed && textarea.selectionDirection === 'backward'
				? textarea.selectionStart
				: textarea.selectionEnd
		const anchor = collapsed
			? caret
			: textarea.selectionDirection === 'backward'
				? textarea.selectionEnd
				: textarea.selectionStart
		for (const m of value.matchAll(pasteTokenRegex())) {
			if (m.index === undefined) continue
			const s = m.index
			const en = m.index + m[0].length
			const target =
				right && caret >= s && caret < en ? en : !right && caret > s && caret <= en ? s : null
			if (target === null) continue
			e.preventDefault()
			if (e.shiftKey) {
				textarea.setSelectionRange(
					Math.min(anchor, target),
					Math.max(anchor, target),
					target < anchor ? 'backward' : 'forward'
				)
			} else {
				textarea.setSelectionRange(target, target)
			}
			return true
		}
		return false
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

	function updateAnchorRect() {
		if (!textarea) return
		try {
			// Index of the `@` that started the current mention. handleInput
			// only opens the picker when `contextTooltipWord` (= `@xxx`) is the
			// LAST whitespace-separated word in `value`, so the `@` always sits
			// at `value.length - contextTooltipWord.length`.
			const atIndex = value.length - contextTooltipWord.length
			const coords = getCaretCoordinates(textarea, atIndex)
			const rect = textarea.getBoundingClientRect()
			anchorRect = new DOMRect(rect.left + coords.left, rect.top + coords.top, 1, coords.height)
			// Re-prime the virtual ref then kick floating-ui (autoUpdate only fires
			// on scroll/resize, not on text changes inside the textarea).
			anchorRef.update({ getBoundingClientRect: anchorRect })
			updateFloating()
		} catch (error) {
			console.error('Error computing anchor rect', error)
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

		// Atomic chip deletion takes precedence over the default char delete.
		if (handlePasteDeletion(e)) {
			return
		}

		if (showContextTooltip) {
			// Forward navigation keys to the picker so the textarea-focused
			// user can drive it. The picker preventDefault/stopPropagation's
			// the ones it handles; we still preventDefault Enter so it never
			// inserts a newline even if no item is highlighted. ArrowLeft /
			// ArrowRight forward too but the picker only consumes them when
			// the search query is empty — so cursor movement within `@xxx`
			// still works once the user has typed past the `@`.
			if (
				e.key === 'ArrowDown' ||
				e.key === 'ArrowUp' ||
				e.key === 'ArrowLeft' ||
				e.key === 'ArrowRight' ||
				e.key === 'Enter' ||
				e.key === 'Tab' ||
				e.key === 'Escape'
			) {
				chatContextPicker?.handleKeydown(e)
			}
			if (e.key === 'Enter') {
				e.preventDefault()
			}
			return
		}

		// Step the caret over a paste chip as one unit (picker closed only).
		if (handlePasteCaretSkip(e)) {
			return
		}

		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			onSendRequest()
		}
	}

	$effect(() => {
		// Re-track on every value change. The `@` position can shift when the
		// user adds/deletes text BEFORE it (line wrap, etc.); the picker should
		// follow. floating-ui's autoUpdate only fires on scroll/resize.
		void value
		if (showContextTooltip) updateAnchorRect()
	})

	$effect(() => {
		// Mention-removal sync: any title that was a `@mention` last frame and
		// is gone now → drop the matching selectedContext entry. Reactive (not
		// inside handleInput) so it catches both keystroke deletions AND any
		// programmatic value changes from the picker insertion path.
		const prev = prevMentionedTitles
		const cur = mentionedTitles
		for (const title of prev) {
			if (cur.has(title)) continue
			const entry = selectedContext.find((c) => c.title === title && c.deletable !== false)
			if (entry) onRemoveContext?.(entry)
		}
		prevMentionedTitles = cur
	})

	export function focus() {
		textarea?.focus()
	}

	// Wipe after dispatching a send: pre-zero `prevMentionedTitles` so the
	// effect above sees no diff when `value` clears, leaving `selectedContext`
	// untouched until `AIChatManager.beforeSend` snapshots it. A manual
	// textarea clear by the user keeps the old behaviour (badges drop).
	export function clearForSend() {
		prevMentionedTitles = new Set()
		value = ''
	}

	// Called by the host BEFORE it strips a mention token from `value`
	// (badge-delete path). Drops the title from `prevMentionedTitles` so when
	// the strip lands, the effect sees no diff and doesn't dispatch another
	// `onRemoveContext` — the host has already mutated `selectedContext`.
	// Critical when two `selectedContext` entries share a title (workspace
	// script + flow with same path): without this, the effect would find the
	// surviving sibling by title and remove it too.
	export function unsyncMention(title: string) {
		if (!prevMentionedTitles.has(title)) return
		const next = new Set(prevMentionedTitles)
		next.delete(title)
		prevMentionedTitles = next
	}
</script>

<div class="relative w-full scroll-pb-2 bg-surface">
	<div
		class={twMerge(
			'textarea-input absolute inset-0 overflow-hidden pointer-events-none',
			CHAT_INPUT_PADDING,
			className
		)}
	>
		<div style="transform: translateY({-scrollTop}px)" use:chipClickDelegate>
			<span class="break-words">
				{@html getHighlightedText(value)}
			</span>
		</div>
	</div>
	<textarea
		bind:this={textarea}
		onkeydown={handleKeyDown}
		bind:value
		use:autosize={{ maxHeight: '40vh' }}
		rows={1}
		oninput={handleInput}
		onpaste={handlePaste}
		onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
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
			CHAT_INPUT_PADDING,
			className
		)}
		class:transparent-text={value.length > 0}
		{disabled}
	></textarea>
</div>

{#if showContextTooltip}
	<Portal target="body">
		<div
			bind:this={tooltipElement}
			use:floatingContent
			class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md shadow-lg overflow-hidden"
			style="z-index: {zIndexes.tooltip};"
		>
			<ChatContextPicker
				bind:this={chatContextPicker}
				{availableContext}
				{selectedContext}
				onSelect={(element) => {
					handleContextSelection(element)
				}}
				onSelectWorkspaceItem={(element) => {
					onAddContext(element)
					updateInstructionsWithContext(element)
					showContextTooltip = false
					setTimeout(() => textarea?.focus(), 0)
				}}
				externalFilter={contextTooltipWord.slice(1)}
				autoFocus={false}
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
		min-height: 2.25rem;
	}

	/* Hide the textarea's own glyphs (the highlight overlay renders the text)
	   while keeping the caret visible. Toggled via a class rather than an inline
	   `style` so it never clobbers the inline height set by the autosize action. */
	.transparent-text {
		color: transparent;
		-webkit-text-fill-color: transparent;
	}
</style>
