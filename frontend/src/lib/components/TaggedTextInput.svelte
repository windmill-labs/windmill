<script lang="ts">
	let {
		tags,
		value = $bindable(''),
		placeholder = '',
		highlights,
		onCurrentTagChange,
		onTextSegmentAtCursorChange,
		class: className = ''
	}: {
		tags: { regex: RegExp; id: string; onClear?: () => void }[]
		value?: string
		placeholder?: string
		highlights?: { regex: RegExp; classes: string }[]
		onCurrentTagChange?: (tag: { id: string } | null) => void
		onTextSegmentAtCursorChange?: (segment: { text: string; start: number; end: number }) => void
		class?: string
	} = $props()

	let contentEditableDiv: HTMLDivElement
	let isUpdating = false

	$effect(() => {
		if (!value.trim() && value !== '') value = ''
	})

	let _preventCursorMoveOnNextSync = false
	// Update the displayed HTML when value changes externally
	$effect(() => {
		if (contentEditableDiv && !isUpdating) {
			const currentText = getTextContent()
			if (currentText !== value) {
				updateDisplay(value)
				if (!_preventCursorMoveOnNextSync) {
					restoreCursor(value.length)
					const cursorPos = getCursorPosition()
					updateCurrentTag(cursorPos)
				}
			}
		}
		_preventCursorMoveOnNextSync = false
	})

	export function preventCursorMoveOnNextSync() {
		_preventCursorMoveOnNextSync = true
	}

	function getTextContent(): string {
		if (!contentEditableDiv) return ''
		return contentEditableDiv.textContent || ''
	}

	function updateDisplay(text: string) {
		if (!contentEditableDiv) return

		const html = highlightText(text)
		contentEditableDiv.innerHTML = html
	}

	/** Apply secondary highlight spans within a raw-text chunk. Returns HTML. */
	function applyHighlightsToChunk(rawText: string): string {
		if (!highlights || highlights.length === 0) return escapeHtml(rawText)

		// Find all highlight matches in the raw text
		const hlMatches: Array<{ start: number; end: number; classes: string }> = []
		for (const hl of highlights) {
			const regex = new RegExp(hl.regex, 'g')
			let m
			while ((m = regex.exec(rawText)) !== null) {
				hlMatches.push({ start: m.index, end: m.index + m[0].length, classes: hl.classes })
			}
		}
		if (hlMatches.length === 0) return escapeHtml(rawText)

		// Sort and deduplicate (keep first on overlap)
		hlMatches.sort((a, b) => a.start - b.start)
		const filtered: typeof hlMatches = []
		let lastEnd = -1
		for (const m of hlMatches) {
			if (m.start >= lastEnd) {
				filtered.push(m)
				lastEnd = m.end
			}
		}

		let result = ''
		let idx = 0
		for (const m of filtered) {
			if (m.start > idx) {
				result += escapeHtml(rawText.slice(idx, m.start))
			}
			result += `<span class="${m.classes}">${escapeHtml(rawText.slice(m.start, m.end))}</span>`
			idx = m.end
		}
		if (idx < rawText.length) {
			result += escapeHtml(rawText.slice(idx))
		}
		return result
	}

	function highlightText(text: string): string {
		if (!text) return ''

		// Create a list of all matches with their positions
		const matches: Array<{ start: number; end: number; tagIndex: number }> = []

		tags.forEach((tag, tagIndex) => {
			const regex = new RegExp(tag.regex, 'g')
			let match
			while ((match = regex.exec(text)) !== null) {
				matches.push({
					start: match.index,
					end: match.index + match[0].length,
					tagIndex
				})
			}
		})

		// Sort matches by start position
		matches.sort((a, b) => a.start - b.start)

		// Remove overlapping matches (keep the first one)
		const filteredMatches: Array<{ start: number; end: number; tagIndex: number }> = []
		let lastEnd = -1
		for (const match of matches) {
			if (match.start >= lastEnd) {
				filteredMatches.push(match)
				lastEnd = match.end
			}
		}

		// Build HTML with highlighted segments
		let html = ''
		let lastIndex = 0

		for (const match of filteredMatches) {
			// Add text before the match (apply secondary highlights)
			if (match.start > lastIndex) {
				html += applyHighlightsToChunk(text.slice(lastIndex, match.start))
			}

			// Add highlighted match (with secondary highlights applied inside)
			const matchedText = text.slice(match.start, match.end)
			const tagId = tags[match.tagIndex].id
			const hasClear = !!tags[match.tagIndex].onClear
			const clearBtn = hasClear
				? `<span data-clear-tag="${tagId}" class="inline-flex w-2.5 h-3 ml-1 cursor-pointer opacity-50 hover:opacity-100" style="vertical-align: middle;"><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" style="pointer-events:none"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg></span>`
				: ''
			html += `<span class="bg-surface-sunken/50 border border-border-light py-0.5 px-1.5 rounded" data-tag-id="${tagId}">${applyHighlightsToChunk(matchedText)}${clearBtn}</span>`

			lastIndex = match.end
		}

		// Add remaining text (apply secondary highlights)
		if (lastIndex < text.length) {
			html += applyHighlightsToChunk(text.slice(lastIndex))
		}

		return html
	}

	function escapeHtml(text: string): string {
		const div = document.createElement('div')
		div.textContent = text
		let html = div.innerHTML
		html = html.replace(/\\(n|r|.)/g, (match, c) => {
			const display = c === 'n' ? '↵' : c === 'r' ? '↵' : c
			return (
				'<span style="display: inline; width: 0; height: 0; overflow: hidden; position: absolute;">\\</span>' +
				display
			)
		})
		return html
	}

	let lastText = ''

	function applyTextUpdate(newText: string, newCursorPos: number) {
		value = newText
		updateDisplay(newText)
		restoreCursor(newCursorPos)
		updateCurrentTag(newCursorPos)
		lastText = newText
		isUpdating = false
	}

	function handleInput() {
		isUpdating = true
		const cursorPos = getCursorPosition()
		let newText = getTextContent()

		// Remove any "\." sequences that were added by browser smart punctuation
		// These would only be created by macOS/browser when user double-presses space
		if (newText.includes('\\.')) {
			const cleanedText = newText.replace(/\\\./g, '')
			const removedCount = (newText.length - cleanedText.length) / 2 // Each "\." is 2 chars
			applyTextUpdate(cleanedText, cursorPos - removedCount * 2)
			return
		}

		// Escape any literal newlines (e.g. from Shift+Enter or IME input)
		if (newText.includes('\n') || newText.includes('\r')) {
			const before = newText.slice(0, cursorPos)
			const newlinesBefore = (before.match(/[\n\r]/g) || []).length
			const cleanedText = newText.replace(/\r\n/g, '\\n').replace(/[\n\r]/g, '\\n')
			// Each newline becomes 2 chars (\n), so cursor shifts by +1 per newline before it
			applyTextUpdate(cleanedText, cursorPos + newlinesBefore)
			return
		}

		// Check if user just typed an escaped character
		if (
			newText.length > lastText.length &&
			(newText[cursorPos - 1] === ' ' ||
				newText[cursorPos - 1] === '\u00A0' ||
				newText[cursorPos - 1] === '\\')
		) {
			// Check if there's already an escaped space right before the cursor (e.g., "tag\ |")
			// If user types another space, just remove the backslash instead of adding "\ \"
			if (
				(newText[cursorPos - 1] === ' ' || newText[cursorPos - 1] === '\u00A0') &&
				newText[cursorPos - 3] === '\\' &&
				(newText[cursorPos - 2] === ' ' || newText[cursorPos - 2] === '\u00A0')
			) {
				// Remove the backslash before the existing space
				newText = newText.slice(0, cursorPos - 3) + newText.slice(cursorPos - 2)
				applyTextUpdate(newText, cursorPos - 1)
				return
			}

			// Escape the space/backslash by adding backslash before it
			newText = newText.slice(0, cursorPos - 1) + '\\' + newText.slice(cursorPos - 1)
			applyTextUpdate(newText, cursorPos + 1)
			return
		}

		applyTextUpdate(newText, cursorPos)
	}

	function getTextSegmentAtCursor(cursorPos: number): {
		text: string
		start: number
		end: number
	} | null {
		// Find all tag positions
		const tagPositions: Array<{ start: number; end: number }> = []
		for (const tag of tags) {
			const regex = new RegExp(tag.regex, 'g')
			let match
			while ((match = regex.exec(value)) !== null) {
				tagPositions.push({
					start: match.index,
					end: match.index + match[0].length
				})
			}
		}

		// Sort by start position
		tagPositions.sort((a, b) => a.start - b.start)

		// Find the text segment containing the cursor
		let segmentStart = 0
		let segmentEnd = value.length

		for (const tag of tagPositions) {
			if (cursorPos <= tag.start) {
				// Cursor is before this tag
				segmentEnd = tag.start
				break
			} else if (cursorPos > tag.end) {
				// Cursor is after this tag
				segmentStart = tag.end
			} else {
				// Cursor is inside a tag
				return null
			}
		}

		return {
			text: value.slice(segmentStart, segmentEnd).trim(),
			start: segmentStart,
			end: segmentEnd
		}
	}

	function updateCurrentTag(cursorPos: number) {
		let currentTag: { id: string } | null = null

		for (const tag of tags) {
			const regex = new RegExp(tag.regex, 'g')
			let match
			while ((match = regex.exec(value)) !== null) {
				const start = match.index
				const end = match.index + match[0].length
				if (cursorPos >= start && cursorPos <= end) {
					currentTag = { id: tag.id }

					onCurrentTagChange?.(currentTag)
					onTextSegmentAtCursorChange?.({ text: '', start: 0, end: 0 })
					return
				}
			}
		}

		onCurrentTagChange?.(null)

		// Get text segment at cursor when not in a tag
		const textSegment = getTextSegmentAtCursor(cursorPos)
		if (textSegment) {
			onTextSegmentAtCursorChange?.(textSegment)
		}
	}

	function handleClick(e: MouseEvent) {
		// Check if the click landed on a clear button
		const target = e.target as HTMLElement | null
		const clearTarget = target?.closest<HTMLElement>('[data-clear-tag]')
		if (clearTarget) {
			const tagId = clearTarget.dataset.clearTag!
			const tag = tags.find((t) => t.id === tagId)
			tag?.onClear?.()
			return
		}
		const cursorPos = getCursorPosition()
		updateCurrentTag(cursorPos)
	}

	function handleKeyup(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter') return
		const cursorPos = getCursorPosition()
		updateCurrentTag(cursorPos)
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Enter') return
		const cursorPos = getCursorPosition()
		const text = getTextContent()

		// Handle Backspace key to remove escape sequences
		if (e.key === 'Backspace') {
			// Check if we're right after an escaped character (e.g., "abc\ |def")
			// We want to remove both the backslash and the escaped character
			if (cursorPos >= 2 && text[cursorPos - 2] === '\\') {
				e.preventDefault()
				isUpdating = true
				const newText = text.slice(0, cursorPos - 2) + text.slice(cursorPos)
				applyTextUpdate(newText, cursorPos - 2)
				return
			}
		}

		// Handle Delete key to remove escape sequences
		if (e.key === 'Delete') {
			// Check if the character at cursor position is a backslash (escape character)
			if (cursorPos < text.length && text[cursorPos] === '\\' && cursorPos + 1 < text.length) {
				e.preventDefault()
				isUpdating = true
				const newText = text.slice(0, cursorPos) + text.slice(cursorPos + 2)
				applyTextUpdate(newText, cursorPos)
				return
			}
		}

		// Handle arrow key navigation to skip escape sequences
		if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
			if (e.key === 'ArrowLeft' && cursorPos > 0) {
				// Moving left: check if we're right after an escaped character (e.g., "abc\ |def")
				// We want to skip over the backslash and the escaped character
				if (cursorPos >= 2 && text[cursorPos - 2] === '\\') {
					e.preventDefault()
					isUpdating = true
					restoreCursor(cursorPos - 2)
					updateCurrentTag(cursorPos - 2)
					isUpdating = false
					return
				}
			} else if (e.key === 'ArrowRight') {
				// Moving right: check if we're at a backslash (e.g., "abc|\ def")
				// We want to skip over the backslash and the escaped character
				if (cursorPos < text.length && text[cursorPos] === '\\' && cursorPos + 1 < text.length) {
					e.preventDefault()
					isUpdating = true
					restoreCursor(cursorPos + 2)
					updateCurrentTag(cursorPos + 2)
					isUpdating = false
					return
				}

				// If user pressed right arrow and is at the end, add a space if needed
				if (
					cursorPos === text.length &&
					text.length > 0 &&
					((text[text.length - 1] !== ' ' && text[text.length - 1] !== '\u00A0') ||
						text[text.length - 2] === '\\')
				) {
					e.preventDefault()
					isUpdating = true
					const newText = text + '\u00A0'
					applyTextUpdate(newText, newText.length)
					return
				}
			}
		}
	}

	function getCursorPosition(): number {
		if (!contentEditableDiv) return 0

		const selection = window.getSelection()
		if (!selection || selection.rangeCount === 0) return 0

		const range = selection.getRangeAt(0)
		const preCaretRange = range.cloneRange()
		preCaretRange.selectNodeContents(contentEditableDiv)
		preCaretRange.setEnd(range.endContainer, range.endOffset)

		return preCaretRange.toString().length
	}

	function restoreCursor(position: number) {
		if (!contentEditableDiv) return

		const selection = window.getSelection()
		if (!selection) return

		let currentPos = 0
		let node: Node | null = null
		let offset = 0

		function traverse(n: Node): boolean {
			if (n.nodeType === Node.TEXT_NODE) {
				const textLength = n.textContent?.length || 0
				if (currentPos + textLength >= position) {
					node = n
					offset = position - currentPos
					return true
				}
				currentPos += textLength
			} else {
				for (let i = 0; i < n.childNodes.length; i++) {
					if (traverse(n.childNodes[i])) {
						return true
					}
				}
			}
			return false
		}

		traverse(contentEditableDiv)

		if (node) {
			const range = document.createRange()
			range.setStart(node, offset)
			range.collapse(true)
			selection.removeAllRanges()
			selection.addRange(range)

			// Ensure cursor is visible by scrolling if needed
			ensureCursorVisible()
		}
	}

	function ensureCursorVisible() {
		if (!contentEditableDiv) return

		const selection = window.getSelection()
		if (!selection || selection.rangeCount === 0) return

		const range = selection.getRangeAt(0)
		const rect = range.getBoundingClientRect()
		const containerRect = contentEditableDiv.getBoundingClientRect()

		// Check if cursor is outside the visible area horizontally
		if (rect.left < containerRect.left) {
			// Cursor is to the left of visible area
			contentEditableDiv.scrollLeft -= containerRect.left - rect.left + 10
		} else if (rect.right > containerRect.right) {
			// Cursor is to the right of visible area
			contentEditableDiv.scrollLeft += rect.right - containerRect.right + 10
		}
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault()
		let text = e.clipboardData?.getData('text/plain') || ''
		// Escape backslashes, spaces, and newlines
		text = text
			.replace(/\\/g, '\\\\')
			.replace(/ /g, '\\ ')
			.replace(/\r\n/g, '\\n')
			.replace(/[\n\r]/g, '\\n')
		document.execCommand('insertText', false, text)
	}

	export function focusAtEnd() {
		if (!contentEditableDiv) return
		contentEditableDiv.focus()
		restoreCursor(value.length)
		updateCurrentTag(value.length)
		contentEditableDiv.scrollLeft = contentEditableDiv.scrollWidth
	}
</script>

<div
	bind:this={contentEditableDiv}
	contenteditable="true"
	oninput={handleInput}
	onpaste={handlePaste}
	onclick={handleClick}
	onkeydown={handleKeyDown}
	onkeyup={handleKeyup}
	class="outline-none text-nowrap pt-[0.45rem] {className}"
	class:text-hint={value === ''}
	data-placeholder={placeholder}
	role="textbox"
	tabindex="0"
	spellcheck="false"
></div>

<style>
	[contenteditable][data-placeholder]:empty::before {
		content: attr(data-placeholder);
	}
</style>
