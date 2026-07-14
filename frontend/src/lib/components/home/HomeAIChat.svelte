<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ArrowUp, Settings } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import ProviderModelSelector from '../copilot/chat/ProviderModelSelector.svelte'
	import { startSessionWithPrompt } from '../sessions/sessionSwitch.svelte'
	import { copilotInfo, copilotWorkspace, loadCopilot } from '$lib/aiStore'
	import { workspaceStore } from '$lib/stores'
	import { base } from '$lib/base'

	let value = $state('')
	let placeholder = $state('')

	// In global-AI mode the layout's chat panel is disabled and never loads the copilot
	// config, so the home chat loads it for the current workspace itself.
	$effect(() => {
		if ($workspaceStore) {
			loadCopilot($workspaceStore)
		}
	})

	// No usable model (no provider configured, or AI disabled): the chat is shown but
	// non-interactive, and hovering reveals a prompt to configure AI in the settings.
	// Gate on the config having actually loaded for this workspace so the initial
	// (unloaded) state doesn't flash the overlay while a provider is configured.
	let disabled = $derived($copilotWorkspace === $workspaceStore && !$copilotInfo.enabled)

	let starting = $state(false)
	async function start() {
		if (disabled || starting || !value.trim()) return
		starting = true
		try {
			await startSessionWithPrompt(value)
		} finally {
			starting = false
		}
	}

	// Enter starts the session; Shift+Enter keeps inserting a newline.
	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault()
			start()
		}
	}

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
	<div class="max-w-[40rem] grow relative group">
		<p class="text-center font-regular text-3xl mb-4">Build with AI</p>
		<div
			class={disabled
				? 'transition-[filter] group-hover:blur-sm pointer-events-none select-none'
				: ''}
		>
			<TextInput
				bind:value
				class="resize-none px-4 py-3 pb-9 shadow-sm border-accent"
				underlyingInputEl="textarea"
				inputProps={{ rows: 4, placeholder, onkeydown: onKeydown }}
			/>
			<Button
				endIcon={starting ? {} : { icon: ArrowUp }}
				wrapperClasses="absolute right-2 bottom-3.5"
				variant={value.trim() ? 'accent' : 'subtle'}
				iconOnly
				loading={starting}
				disabled={!value.trim() || starting || disabled}
				onclick={start}
			></Button>
			<div
				class="absolute left-3 bottom-4 flex items-center border rounded-md bg-surface-tertiary px-1"
			>
				<ProviderModelSelector />
			</div>
		</div>
		{#if disabled}
			<div
				class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 rounded-md bg-surface/70 opacity-0 transition-opacity pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto"
			>
				<p class="text-sm text-secondary">No AI provider is configured</p>
				<Button
					unifiedSize="sm"
					variant="accent"
					startIcon={{ icon: Settings }}
					href="{base}/workspace_settings?tab=ai"
				>
					Configure AI
				</Button>
			</div>
		{/if}
	</div>
</div>
