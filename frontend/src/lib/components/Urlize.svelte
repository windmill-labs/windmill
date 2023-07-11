<script lang="ts">
	export let text: string
	const parseURL = (input) => {
		if (input.startsWith('http://') || input.startsWith('https://')) {
			return `<a href="${input}" target="_blank" rel="noopener noreferrer">${input}</a>`
		} else {
			return input
		}
	}

	const parseInput = (input) => {
		if (!input) return ''
		const words = input.split(' ')
		return words.map((word) => parseURL(word)).join(' ')
	}

	$: parsed = text ? text.split('\n').map(parseInput).join('\n') : ''
</script>

{@html parsed}
