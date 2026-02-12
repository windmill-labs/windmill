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

	function handleInput() {
		isUpdating = true
		const cursorPos = getCursorPosition()
		const newText = getTextContent()

		value = newText
		updateDisplay(newText)
		restoreCursor(cursorPos)
		updateCurrentTag(cursorPos)
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
		if (!onCurrentTagChange) return
		let currentTag: { id: string } | null = null

		for (const tag of tags) {
			const regex = new RegExp(tag.regex, 'g')
			let match
			while ((match = regex.exec(value)) !== null) {
				const start = match.index
				const end = match.index + match[0].length
				if (cursorPos >= start && cursorPos <= end) {
					currentTag = { id: tag.id }
					onCurrentTagChange(currentTag)
					onTextSegmentAtCursorChange?.({ text: '', start: 0, end: 0 })
					return
				}
			}
		}
		onCurrentTagChange(null)

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

	function handleKeyup() {
		const cursorPos = getCursorPosition()
		updateCurrentTag(cursorPos)
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
