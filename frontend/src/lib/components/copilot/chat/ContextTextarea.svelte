<script lang="ts">
	import autosize from '$lib/autosize'
	import { tick, type Snippet } from 'svelte'
	import type { ContextElement } from './context'
	import { AIMode } from './AIChatManager.svelte'
	import ChatCommandPicker from './ChatCommandPicker.svelte'
	import ChatContextPicker from './ChatContextPicker.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { twMerge } from 'tailwind-merge'
	import { CHAT_INPUT_PADDING, getAiChatManager } from './aiChatManagerContext'
	import { MENTION_RE, mentionTitle, formatMention } from './mention'
	import { createFloatingActions, createVirtualElement } from 'svelte-floating-ui'
	import { flip, offset, shift } from 'svelte-floating-ui/dom'
	import {
		type PasteAttachment,
		countLines,
		expandPasteTokens,
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
		/** Called with image files found in a paste, so the host can attach them. */
		onImageFiles?: (files: File[]) => void
		/** Called with non-image files found in a paste, so the host can attach them. */
		onTextFiles?: (files: File[]) => void
		className?: string
		onKeyDown?: (e: KeyboardEvent) => void
		/** Rendered inside the input box, above the textarea (e.g. context chips). */
		leading?: Snippet
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
		onImageFiles,
		onTextFiles,
		className = '',
		onKeyDown = undefined,
		leading
	}: Props = $props()

	const aiChatManager = getAiChatManager()

	function extractMentions(text: string): Set<string> {
		const out = new Set<string>()
		for (const m of text.matchAll(MENTION_RE)) out.add(mentionTitle(m[0]))
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
	let showCommandTooltip = $state(false)
	let commandTooltipWord = $state('')
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)
	let tooltipElement = $state<HTMLDivElement | undefined>(undefined)
	let chatContextPicker: ChatContextPicker | undefined = $state()
	let chatCommandPicker: ChatCommandPicker | undefined = $state()
	let commandSkillsRefreshInFlight = false

	const commandSkills = $derived(
		aiChatManager.mode === AIMode.GLOBAL && aiChatManager.isSessionChat
			? aiChatManager.sessionCommands
			: []
	)
	const activeTooltipWord = $derived(showContextTooltip ? contextTooltipWord : commandTooltipWord)

	// Virtual reference anchored at the trigger that opened the picker (not the
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

	// Inverse of escapeHtml (&amp; last so an escaped entity isn't double-decoded). Mentions
	// are parsed out of the escaped HTML, so a title is un-escaped before the store lookup —
	// else a filename like `R&D notes.md` (escaped to `R&amp;D notes.md`) would never match.
	function unescapeHtml(text: string) {
		return text
			.replace(/&lt;/g, '<')
			.replace(/&gt;/g, '>')
			.replace(/&quot;/g, '"')
			.replace(/&#39;/g, "'")
			.replace(/&amp;/g, '&')
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
		html = html.replace(MENTION_RE, (match) => {
			const title = unescapeHtml(mentionTitle(match))
			const inContext =
				availableContext.find((c) => c.title === title) ||
				selectedContext.find((c) => c.title === title) ||
				// Attached-file mentions (`@filename`) highlight just like context.
				aiChatManager.attachedFiles.get(title)
			if (inContext) {
				return `<span class="bg-surface-accent-selected text-primary rounded box-decoration-clone z-10">${match}</span>`
			}
			return match
		})
		return html
	}

	// Find the paste tokens that overlap a [start, end) selection, returning the
	// range widened to cover each overlapped token whole (so a chip is never cut
	// mid-token) plus the ids to drop from the registry. Strict overlap — merely
	// abutting a token edge doesn't pull it in.
	function tokensOverlapping(start: number, end: number) {
		let from = start
		let to = end
		const ids: number[] = []
		for (const m of value.matchAll(pasteTokenRegex())) {
			if (m.index === undefined) continue
			const tokenStart = m.index
			const tokenEnd = m.index + m[0].length
			if (tokenStart < end && tokenEnd > start) {
				from = Math.min(from, tokenStart)
				to = Math.max(to, tokenEnd)
				ids.push(m[1].length)
			}
		}
		return { from, to, ids }
	}

	// On a large paste, register the blob and insert a compact token instead of
	// the raw lines (see pasteTokens.ts). Smaller pastes fall through to default
	// (and beforeinput keeps any overlapped chip atomic). The insertion range is
	// widened over overlapped tokens so pasting onto a chip replaces it whole.
	function handlePaste(e: ClipboardEvent) {
		const text = e.clipboardData?.getData('text/plain') ?? ''
		// Image paste (screenshots, copied images) → hand off to the host to attach.
		// Only when the clipboard carries no text: spreadsheet and browser copies put a
		// bitmap alongside the text, and pasting a cell range must paste the cells, not
		// a picture of them. An OS screenshot carries the image alone, so it still lands
		// here. `onImageFiles` is unset outside GLOBAL, where attaching is unsupported —
		// the paste must then fall through to text rather than be swallowed.
		if (!text.trim() && (onImageFiles || onTextFiles)) {
			const pastedFiles = Array.from(e.clipboardData?.files ?? [])
			const imageFiles = pastedFiles.filter((f) => f.type.startsWith('image/'))
			const otherFiles = pastedFiles.filter((f) => !f.type.startsWith('image/'))
			if ((imageFiles.length > 0 && onImageFiles) || (otherFiles.length > 0 && onTextFiles)) {
				e.preventDefault()
				if (imageFiles.length > 0) onImageFiles?.(imageFiles)
				if (otherFiles.length > 0) onTextFiles?.(otherFiles)
				return
			}
		}
		if (!text || !shouldCollapsePaste(text)) return
		e.preventDefault()
		const ta = e.currentTarget as HTMLTextAreaElement
		const { from, to, ids } = tokensOverlapping(
			ta.selectionStart ?? value.length,
			ta.selectionEnd ?? value.length
		)
		const att: PasteAttachment = {
			id: nextPasteId(pastes ?? []),
			lines: countLines(text),
			content: text
		}
		const token = makePasteToken(att)
		value = value.slice(0, from) + token + value.slice(to)
		pastes = [...(pastes ?? []).filter((p) => !ids.includes(p.id)), att]
		const caret = from + token.length
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

	// Replace the [from, to) range with `insert`, drop the given paste ids, and
	// place the caret after the inserted text — keeping value, the pastes
	// registry, and the caret consistent.
	function replacePasteRange(from: number, to: number, ids: number[], insert = '') {
		value = value.slice(0, from) + insert + value.slice(to)
		pastes = (pastes ?? []).filter((p) => !ids.includes(p.id))
		const caret = from + insert.length
		tick().then(() => textarea?.setSelectionRange(caret, caret))
	}

	// Keep chip deletion atomic for Backspace/Delete with a collapsed caret at a
	// token boundary (the one case beforeinput can't see, since nothing is
	// selected). Selection-spanning edits — including Backspace/Delete over a
	// selection — are handled uniformly in handlePasteBeforeInput.
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
				replacePasteRange(tokenStart, tokenEnd, [m[1].length])
				return true
			}
		}
		return false
	}

	// The content-mutating inputTypes we know how to reproduce. We intercept ONLY
	// these — never history (historyUndo/Redo, which we'd turn into a deletion)
	// nor composition (insertCompositionText is non-cancelable, so preventDefault
	// is a no-op while we'd still rewrite value mid-IME).
	const HANDLED_INPUT_TYPES = new Set([
		'insertText',
		'insertReplacementText',
		'insertFromYank',
		'insertFromPaste',
		'insertFromDrop',
		'insertLineBreak',
		'insertParagraph',
		'deleteContentBackward',
		'deleteContentForward',
		'deleteContent',
		'deleteByCut',
		'deleteByDrag',
		'deleteWordBackward',
		'deleteWordForward',
		'deleteSoftLineBackward',
		'deleteSoftLineForward',
		'deleteHardLineBackward',
		'deleteHardLineForward'
	])

	// Any selection-spanning input (typing over a selection, paste, cut,
	// drag-and-drop, Backspace/Delete over a selection) that overlaps a chip is
	// taken over and applied to the whole token(s), so a partial edit can never
	// leave an orphaned zero-width run or a dangling pastes entry. Backspace/
	// Delete at a collapsed boundary is handled earlier in keydown.
	function handlePasteBeforeInput(e: InputEvent) {
		// Skip non-cancelable events (e.g. IME composition) — we can't suppress
		// them, and unknown inputTypes (history) we mustn't reinterpret as a delete.
		if (!textarea || !e.cancelable || !HANDLED_INPUT_TYPES.has(e.inputType)) return
		const start = textarea.selectionStart
		const end = textarea.selectionEnd
		if (start === null || end === null || start === end) return
		const { from, to, ids } = tokensOverlapping(start, end)
		if (ids.length === 0) return
		e.preventDefault()
		replacePasteRange(from, to, ids, insertedText(e))
	}

	// The text a (whitelisted) beforeinput event will insert, by inputType.
	// Deletions insert nothing; line breaks insert a newline; paste/drop/yank/
	// replacement carry their payload on dataTransfer, plain typing on `data`.
	function insertedText(e: InputEvent): string {
		const t = e.inputType
		if (t.startsWith('delete')) return ''
		if (t === 'insertLineBreak' || t === 'insertParagraph') return '\n'
		if (
			t === 'insertFromPaste' ||
			t === 'insertFromDrop' ||
			t === 'insertReplacementText' ||
			t === 'insertFromYank'
		) {
			return e.dataTransfer?.getData('text/plain') ?? e.data ?? ''
		}
		return e.data ?? ''
	}

	// Dragging a selection that contains a chip carries the *expanded* content on
	// the drag payload (mirroring copy/cut), so dropping it — inside the input or
	// onto an external target — yields the real text, never the chip label + its
	// zero-width id run (which, once its registry entry is gone, can't resolve).
	function handlePasteDragStart(e: DragEvent) {
		if (!textarea || !e.dataTransfer) return
		const start = textarea.selectionStart
		const end = textarea.selectionEnd
		if (start === null || end === null || start === end) return
		const { from, to, ids } = tokensOverlapping(start, end)
		if (ids.length === 0) return
		e.dataTransfer.setData('text/plain', expandPasteTokens(value.slice(from, to), pastes ?? []))
	}

	// Copy/cut of a selection containing a chip puts the *expanded* content on the
	// clipboard instead of the chip label + its invisible zero-width run; cut also
	// removes the chip whole. Selections without a chip use the browser default.
	function handlePasteCopyCut(e: ClipboardEvent) {
		if (!textarea || !e.clipboardData) return
		const start = textarea.selectionStart
		const end = textarea.selectionEnd
		if (start === null || end === null || start === end) return
		const { from, to, ids } = tokensOverlapping(start, end)
		if (ids.length === 0) return
		e.preventDefault()
		e.clipboardData.setData('text/plain', expandPasteTokens(value.slice(from, to), pastes ?? []))
		if (e.type === 'cut') {
			replacePasteRange(from, to, ids)
		}
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

	function refreshCommandSkills() {
		if (commandSkillsRefreshInFlight) return
		commandSkillsRefreshInFlight = true
		void aiChatManager.refreshGlobalSkills().finally(() => {
			commandSkillsRefreshInFlight = false
		})
	}

	function getCommandFilter(text: string): string | undefined {
		if (aiChatManager.mode !== AIMode.GLOBAL || !aiChatManager.isSessionChat) return undefined
		const match = /^\/([a-z0-9-]*)$/.exec(text)
		return match?.[1]
	}

	function updateAnchorRect() {
		if (!textarea) return
		const triggerWord = activeTooltipWord
		if (!triggerWord) return
		try {
			// Inline `@` anchors to the last word; slash commands only open when
			// `/...` is the whole input, so the trigger sits at index 0.
			const triggerIndex = triggerWord.startsWith('/') ? 0 : value.length - triggerWord.length
			const coords = getCaretCoordinates(textarea, triggerIndex)
			const rect = textarea.getBoundingClientRect()
			// getCaretCoordinates returns content-relative coords; subtract the
			// textarea's own scroll so the anchor tracks the trigger once the input
			// is capped (max-height) and scrolls internally.
			anchorRect = new DOMRect(
				rect.left + coords.left - textarea.scrollLeft,
				rect.top + coords.top - textarea.scrollTop,
				1,
				coords.height
			)
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

		const commandFilter = getCommandFilter(value)
		if (commandFilter !== undefined) {
			const wasShowing = showCommandTooltip
			showCommandTooltip = true
			commandTooltipWord = `/${commandFilter}`
			showContextTooltip = false
			contextTooltipWord = ''
			if (!wasShowing) refreshCommandSkills()
			return
		}
		showCommandTooltip = false
		commandTooltipWord = ''

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

	function handleCommandSelection(skill: { name: string }) {
		value = `/${skill.name} `
		showCommandTooltip = false
		setTimeout(() => textarea?.focus(), 0)
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

		if (showCommandTooltip) {
			if (
				e.key === 'ArrowDown' ||
				e.key === 'ArrowUp' ||
				e.key === 'Enter' ||
				e.key === 'Tab' ||
				e.key === 'Escape'
			) {
				chatCommandPicker?.handleKeydown(e)
			}
			if (e.key === 'Enter') {
				e.preventDefault()
			}
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
		// Re-track on every value change. The trigger position can shift when
		// the user adds/deletes text before it (line wrap, etc.); the picker
		// should follow. floating-ui's autoUpdate only fires on scroll/resize.
		void value
		if (showContextTooltip || showCommandTooltip) updateAnchorRect()
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

<!-- The composer box: border + rounded live HERE (on the wrapper), not on the
     textarea, so context chips can sit INSIDE the box, above the text. The
     textarea's own @tailwindcss/forms border/ring is neutralized below. -->
<div
	class="w-full scroll-pb-2 bg-surface-input rounded-md border border-border-light focus-within:border-border-selected transition-colors"
>
	<!-- Context chips live inside the input box, above the textarea. The snippet
	     self-guards (renders nothing when empty) so no blank row appears. -->
	{@render leading?.()}
	<div class="relative w-full">
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
			onbeforeinput={handlePasteBeforeInput}
			oncopy={handlePasteCopyCut}
			oncut={handlePasteCopyCut}
			ondragstart={handlePasteDragStart}
			onscroll={(e) => {
				scrollTop = e.currentTarget.scrollTop
				// Keep the picker pinned to its anchor while the input scrolls
				// internally (autoUpdate can't observe a virtual ref's scroll).
				if (showContextTooltip || showCommandTooltip) updateAnchorRect()
			}}
			onblur={() => {
				setTimeout(() => {
					// Don't close if focus moved to inside the tooltip (e.g., search input)
					if (tooltipElement?.contains(document.activeElement)) {
						return
					}
					showContextTooltip = false
					showCommandTooltip = false
				}, 200)
			}}
			{placeholder}
			class={twMerge(
				'textarea-input resize-none caret-black dark:caret-white overflow-clip',
				// The box (border/ring) lives on the wrapper; kill the textarea's own
				// @tailwindcss/forms border, focus ring, and background so only the
				// wrapper reads as the field.
				'!border-transparent !bg-transparent !shadow-none focus:!border-transparent focus:!ring-0',
				CHAT_INPUT_PADDING,
				className
			)}
			class:transparent-text={value.length > 0}
			{disabled}
		></textarea>
	</div>
</div>

{#if showContextTooltip || showCommandTooltip}
	<Portal target="body">
		<div
			bind:this={tooltipElement}
			use:floatingContent
			class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md shadow-lg overflow-hidden"
			style="z-index: {zIndexes.tooltip};"
		>
			{#if showCommandTooltip}
				<ChatCommandPicker
					bind:this={chatCommandPicker}
					skills={commandSkills}
					onSelect={handleCommandSelection}
					externalFilter={commandTooltipWord.slice(1)}
					autoFocus={false}
					setShowing={(showing) => {
						showCommandTooltip = showing
					}}
				/>
			{:else}
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
					onSelectFile={(name) => {
						// Replace the in-progress `@word` with the chosen mention (bracketed if the
						// filename has spaces, so the highlighter captures it whole).
						const index = value.lastIndexOf('@')
						value = (index !== -1 ? value.substring(0, index) : value) + `${formatMention(name)} `
						showContextTooltip = false
						setTimeout(() => textarea?.focus(), 0)
					}}
				/>
			{/if}
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
