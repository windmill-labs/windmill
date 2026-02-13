<script lang="ts">
	let {
		tags,
		value = $bindable(''),
		placeholder = '',
		onCurrentTagChange,
		onTextSegmentAtCursorChange,
		class: className = ''
	}: {
		tags: { regex: RegExp; id: string }[]
		value?: string
		placeholder?: string
		onCurrentTagChange?: (tag: { id: string } | null) => void
		onTextSegmentAtCursorChange?: (segment: { text: string; start: number; end: number }) => void
		class?: string
	} = $props()

	let contentEditableDiv: HTMLDivElement
	let isUpdating = false

	$effect(() => {
		if (!value.trim() && value !== '') value = ''
	})

	// Update the displayed HTML when value changes externally
	$effect(() => {
		if (contentEditableDiv && !isUpdating) {
			const currentText = getTextContent()
			if (currentText !== value) {
				updateDisplay(value)
				restoreCursor(value.length)
				const cursorPos = getCursorPosition()
				updateCurrentTag(cursorPos)
			}
		}
	})

	function getTextContent(): string {
		if (!contentEditableDiv) return ''
		return contentEditableDiv.textContent || ''
	}

	function updateDisplay(text: string) {
		if (!contentEditableDiv) return

		const html = highlightText(text)
		contentEditableDiv.innerHTML = html
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
			// Add text before the match
			if (match.start > lastIndex) {
				html += escapeHtml(text.slice(lastIndex, match.start))
			}

			// Add highlighted match
			const matchedText = text.slice(match.start, match.end)
			html += `<span class="bg-surface-sunken border py-0.5 px-1.5 rounded">${escapeHtml(matchedText)}</span>`

			lastIndex = match.end
		}

		// Add remaining text
		if (lastIndex < text.length) {
			html += escapeHtml(text.slice(lastIndex))
		}

		return html
	}

	function escapeHtml(text: string): string {
		const div = document.createElement('div')
		div.textContent = text
		let html = div.innerHTML
		html = html.replace(/\\./g, (match) => {
			return (
				'<span style="display: inline; width: 0; height: 0; overflow: hidden; position: absolute;">\\</span>' +
				match[1]
			)
		})
		return html
	}

	let lastText = ''

	function handleInput() {
		isUpdating = true
		const cursorPos = getCursorPosition()
		let newText = getTextContent()

		// Remove any "\." sequences that were added by browser smart punctuation
		// These would only be created by macOS/browser when user double-presses space
		if (newText.includes('\\.')) {
			const cleanedText = newText.replace(/\\\./g, '')
			const removedCount = (newText.length - cleanedText.length) / 2 // Each "\." is 2 chars
			newText = cleanedText
			value = newText
			updateDisplay(newText)
			restoreCursor(cursorPos - removedCount * 2)
			updateCurrentTag(cursorPos - removedCount * 2)
			lastText = newText
			isUpdating = false
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
				value = newText
				updateDisplay(newText)
				restoreCursor(cursorPos - 1)
				updateCurrentTag(cursorPos - 1)
				lastText = newText
				isUpdating = false
				return
			}

			// Escape the space/backslash by adding backslash before it
			newText = newText.slice(0, cursorPos - 1) + '\\' + newText.slice(cursorPos - 1)
			value = newText
			updateDisplay(newText)
			restoreCursor(cursorPos + 1)
			updateCurrentTag(cursorPos + 1)
			lastText = newText
			isUpdating = false
			return
		}

		value = newText
		updateDisplay(newText)
		restoreCursor(cursorPos)
		updateCurrentTag(cursorPos)
		lastText = newText
		isUpdating = false
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

	function handleClick() {
		const cursorPos = getCursorPosition()
		updateCurrentTag(cursorPos)
	}

	function handleKeyup(e: KeyboardEvent) {
		const cursorPos = getCursorPosition()
		updateCurrentTag(cursorPos)
	}

	function handleKeyDown(e: KeyboardEvent) {
		const cursorPos = getCursorPosition()
		const text = getTextContent()

		// Handle Backspace key to remove escape sequences
		if (e.key === 'Backspace') {
			// Check if we're right after an escaped character (e.g., "abc\ |def")
			// We want to remove both the backslash and the escaped character
			if (cursorPos >= 2 && text[cursorPos - 2] === '\\') {
				e.preventDefault()
				isUpdating = true
				// Remove both the backslash and the escaped character
				const newText = text.slice(0, cursorPos - 2) + text.slice(cursorPos)
				value = newText
				updateDisplay(newText)
				restoreCursor(cursorPos - 2)
				updateCurrentTag(cursorPos - 2)
				lastText = newText
				isUpdating = false
				return
			}
		}

		// Handle Delete key to remove escape sequences
		if (e.key === 'Delete') {
			// Check if the character at cursor position is a backslash (escape character)
			if (cursorPos < text.length && text[cursorPos] === '\\' && cursorPos + 1 < text.length) {
				e.preventDefault()
				isUpdating = true
				// Remove both the backslash and the escaped character
				const newText = text.slice(0, cursorPos) + text.slice(cursorPos + 2)
				value = newText
				updateDisplay(newText)
				restoreCursor(cursorPos)
				updateCurrentTag(cursorPos)
				lastText = newText
				isUpdating = false
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
					value = newText
					updateDisplay(newText)
					restoreCursor(newText.length)
					updateCurrentTag(newText.length)
					lastText = newText
					isUpdating = false
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
		}
	}

	function handlePaste(e: ClipboardEvent) {
		e.preventDefault()
		let text = e.clipboardData?.getData('text/plain') || ''
		// Escape slashes and spaces
		text = text.replace(/\\/g, '\\\\').replace(/ /g, '\\ ')
		document.execCommand('insertText', false, text)
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
></div>

<style>
	[contenteditable][data-placeholder]:empty::before {
		content: attr(data-placeholder);
	}
</style>
