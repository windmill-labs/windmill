<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ArrowRight, PlusIcon, Sparkles } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import ProviderModelSelector from '../copilot/chat/ProviderModelSelector.svelte'

	// UI only for now — no submit wiring yet.
	let value = $state('')
	let placeholder = $state('')

	const prompts = [
		'Sync new Salesforce leads into a postgres table every hour',
		'Build a workflow that triggers on a Discord message, checks for offensive language using an LLM, and possibly block them'
	]

	const TYPE_MS = 45
	const DELETE_MS = 25
	const HOLD_MS = 1800
	const PAUSE_MS = 400

	// Typewriter effect: type a prompt out, hold, delete, then advance to the next.
	$effect(() => {
		let promptIndex = 0
		let charIndex = 0
		let deleting = false
		let timer: ReturnType<typeof setTimeout>

		function tick() {
			const current = prompts[promptIndex]
			if (!deleting) {
				charIndex++
				placeholder = current.slice(0, charIndex)
				if (charIndex >= current.length) {
					deleting = true
					timer = setTimeout(tick, HOLD_MS)
					return
				}
				timer = setTimeout(tick, TYPE_MS)
			} else {
				charIndex--
				placeholder = current.slice(0, charIndex)
				if (charIndex <= 0) {
					deleting = false
					promptIndex = (promptIndex + 1) % prompts.length
					timer = setTimeout(tick, PAUSE_MS)
					return
				}
				timer = setTimeout(tick, DELETE_MS)
			}
		}

		timer = setTimeout(tick, TYPE_MS)
		return () => clearTimeout(timer)
	})
</script>

<div class="w-full flex justify-center">
	<div class="max-w-[40rem] grow relative">
		<TextInput
			bind:value
			class="resize-none px-4 py-3 pb-9 shadow-lg"
			underlyingInputEl="textarea"
			inputProps={{ rows: 4, placeholder }}
		/>
		<Button
			endIcon={{ icon: ArrowRight }}
			wrapperClasses="absolute right-2 bottom-3.5"
			variant="default"
			iconOnly
		></Button>
		<div
			class="absolute left-3 bottom-4 flex items-center border rounded-md bg-surface-tertiary px-1"
		>
			<ProviderModelSelector />
		</div>
	</div>
</div>
