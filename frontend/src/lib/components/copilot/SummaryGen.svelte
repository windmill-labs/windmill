<script lang="ts">
	import { Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getCompletion } from './lib'
	import type { NewScript } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	const SYSTEM_PORMPT = `
You are a helpful AI assistant. You generate summaries from scripts.
The summaries are very short and give a global idea. Do not specify the programming language. Do not use any punctation.
Examples: List the commits of a GitHub repository, Divide a number by 16, etc..
`

	const PROMPT = `
Generate a very short summary for the script below:
\'\'\'{lang}
{code}
\`\`\`
`

	export let summaryInput: HTMLInputElement | undefined
	export let summary: string
	export let code: string
	export let lang: NewScript.language

	let loading = false

	let abortController = new AbortController()

	async function generateSummary() {
		abortController = new AbortController()
		loading = true
		try {
			const response = await getCompletion(
				[
					{
						role: 'system',
						content: SYSTEM_PORMPT
					},
					{
						role: 'user',
						content: PROMPT.replace('{lang}', lang).replace('{code}', code)
					}
				],
				abortController
			)
			summary = ''
			for await (const part of response) {
				summary += part.choices[0]?.delta?.content || ''
				updateScroll()
			}
		} catch (err) {
			if (abortController.signal.aborted) {
				return
			}
			sendUserToast(`Could not generate summary: ${err}`, true)
		} finally {
			loading = false
		}
	}

	let lastScrollLeft = 0
	function updateScroll() {
		console.log('updateScroll', summaryInput, summaryInput?.scrollWidth, lastScrollLeft)
		if (summaryInput && summaryInput.scrollWidth > lastScrollLeft) {
			console.log('update scroll')
			summaryInput.scrollLeft = summaryInput.scrollWidth
			lastScrollLeft = summaryInput.scrollLeft
		}
	}
</script>

<div class="relative">
	<slot {loading} />
	<Button
		on:click={loading ? () => abortController.abort() : generateSummary}
		wrapperClasses="absolute right-1 top-1/2 -translate-y-1/2 "
		title="Generate Summary"
		iconOnly={!loading}
		variant="contained"
		color={loading ? 'red' : 'light'}
		size="xs2"
		{loading}
		clickableWhileLoading
		startIcon={{ icon: Wand2 }}
	>
		Stop</Button
	>
</div>
