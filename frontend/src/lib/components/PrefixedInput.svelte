<script lang="ts">
	let {
		prefix = '',
		value = $bindable(''),
		placeholder = '',
		class: className = '',
		...restProps
	} = $props()

	let inputElement: HTMLInputElement = $state(null!)
	let internalValue = $state(prefix + value)

	// Update internal value when prop changes
	$effect(() => {
		internalValue = prefix + value
	})

	function handleInput(e) {
		const newValue = e.target.value

		// Ensure the value always starts with the prefix
		if (newValue.startsWith(prefix)) {
			internalValue = newValue
			value = newValue.slice(prefix.length)
		} else {
			internalValue = prefix
			value = ''
			// Reset cursor position
			if (inputElement) {
				inputElement.value = prefix
				inputElement.setSelectionRange(prefix.length, prefix.length)
			}
		}
	}

	function handleKeyDown(e) {
		const cursorPos = e.target.selectionStart
		const selectionEnd = e.target.selectionEnd

		// Prevent backspace if cursor is at or before prefix end
		if (e.key === 'Backspace') {
			if (cursorPos <= prefix.length && selectionEnd <= prefix.length) {
				e.preventDefault()
			} else if (cursorPos <= prefix.length && selectionEnd > prefix.length) {
				// If selection spans across prefix, only delete after prefix
				e.preventDefault()
				const newValue = prefix + internalValue.slice(selectionEnd)
				internalValue = newValue
				value = newValue.slice(prefix.length)
				// Set cursor position after prefix
				setTimeout(() => {
					inputElement.setSelectionRange(prefix.length, prefix.length)
				}, 0)
			}
		}

		// Prevent delete key within prefix
		if (e.key === 'Delete' && cursorPos < prefix.length) {
			e.preventDefault()
		}

		// Prevent selecting all (Ctrl+A) from selecting the prefix
		if ((e.ctrlKey || e.metaKey) && e.key === 'a') {
			e.preventDefault()
			inputElement.setSelectionRange(prefix.length, internalValue.length)
		}
	}

	function handleClick() {
		// Prevent cursor from being placed within prefix
		if ((inputElement.selectionStart ?? 0) < prefix.length) {
			inputElement.setSelectionRange(prefix.length, prefix.length)
		}
	}

	function handleFocus() {
		// Ensure cursor starts after prefix when focusing
		if ((inputElement.selectionStart ?? 0) < prefix.length) {
			inputElement.setSelectionRange(prefix.length, prefix.length)
		}
	}

	function handlePaste(e) {
		e.preventDefault()
		const pasteData = e.clipboardData.getData('text')
		const cursorPos = inputElement.selectionStart ?? 0
		const selectionEnd = inputElement.selectionEnd ?? 0

		if (cursorPos < prefix.length) {
			// Paste at the end of prefix
			internalValue = prefix + pasteData + internalValue.slice(prefix.length)
		} else {
			// Normal paste
			internalValue =
				internalValue.slice(0, cursorPos) + pasteData + internalValue.slice(selectionEnd)
		}

		value = internalValue.slice(prefix.length)

		// Set cursor position after pasted content
		const newCursorPos = Math.max(prefix.length, cursorPos) + pasteData.length
		setTimeout(() => {
			inputElement.setSelectionRange(newCursorPos, newCursorPos)
		}, 0)
	}
</script>

<input
	bind:this={inputElement}
	type="text"
	{placeholder}
	class="prefixed-input {className}"
	value={internalValue}
	oninput={handleInput}
	onkeydown={handleKeyDown}
	onclick={handleClick}
	onfocus={handleFocus}
	onpaste={handlePaste}
	{...restProps}
/>

<style>
	.prefixed-input {
		/* Default styles - can be overridden by class prop */
		padding: 8px 12px;
		border: 1px solid #ccc;
		border-radius: 4px;
		font-size: 14px;
		width: 100%;
		box-sizing: border-box;
	}

	.prefixed-input:focus {
		outline: none;
		border-color: #4caf50;
		box-shadow: 0 0 0 2px rgba(76, 175, 80, 0.1);
	}
</style>
