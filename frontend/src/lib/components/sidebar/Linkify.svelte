<script lang="ts">
	let { text } = $props()
	// Function to convert text with URLs to HTML
	function linkifyText(text) {
		// Regex to match URLs starting with http/https and ending with )
		const urlRegex = /\(((https?:\/\/[^\s)]+))\)/g

		// Replace matched patterns with anchor tags
		return text.replace(urlRegex, '(<a href="$1" target="_blank" rel="noopener noreferrer">$1</a>)')
	}

	let processedText = $derived(linkifyText(text))
</script>

<!-- Use @html to render the HTML -->
{@html processedText}
