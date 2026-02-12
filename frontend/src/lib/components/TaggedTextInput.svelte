<script lang="ts">
	let {
		tags,
		value = $bindable(''),
		placeholder = '',
		class: className = ''
	}: {
		tags: { regex: RegExp }[]
		value?: string
		placeholder?: string
		class?: string
	} = $props()

	let contentEditableDiv: HTMLDivElement
	let isUpdating = false

	// Update the displayed HTML when value changes externally
	$effect(() => {
		if (contentEditableDiv && !isUpdating) {
			const currentText = getTextContent()
			if (currentText !== value) {
				updateDisplay(value)
				restoreCursor(value.length)
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
			html += `<span class="bg-blue-100 dark:bg-blue-900 px-1 rounded">${escapeHtml(matchedText)}</span>`

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
		return div.innerHTML
	}

	function handleInput() {
		isUpdating = true
		const cursorPos = getCursorPosition()
		const newText = getTextContent()

		value = newText
		updateDisplay(newText)
		restoreCursor(cursorPos)
		isUpdating = false
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
		const text = e.clipboardData?.getData('text/plain') || ''
		document.execCommand('insertText', false, text)
	}
</script>

<div
	bind:this={contentEditableDiv}
	contenteditable="true"
	oninput={handleInput}
	onpaste={handlePaste}
	class="outline-none {className}"
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
