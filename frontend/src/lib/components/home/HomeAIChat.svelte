<script lang="ts" module>
	/** Example prompts: the short `label` is shown as a clickable tag under the chat,
	 * the `prompt` is what gets typed out as the placeholder / dropped into the input. */
	export const homeAIExamples: { label: string; prompt: string }[] = [
		{
			label: 'Sync Salesforce',
			prompt: 'Sync new Salesforce leads into a postgres table every hour'
		},
		{
			label: 'Ban Discord users',
			prompt:
				'Build a workflow that triggers on a Discord message, checks for offensive language using an LLM, and possibly block them'
		},
		{
			label: 'Weekly Slack report',
			prompt: 'Generate a weekly sales report from postgres and post it to Slack every Monday'
		}
	]
</script>

<script lang="ts">
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { ArrowUp, ExternalLink, Globe2, KeyRound, PlugZap, Settings } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { startSessionWithPrompt } from '../sessions/sessionSwitch.svelte'
	import { copilotInfo, copilotWorkspace, loadCopilot } from '$lib/aiStore'
	import { hubBaseUrlStore, userStore, workspaceStore } from '$lib/stores'
	import { HOME_SHOW_HUB } from '$lib/consts'
	import { base } from '$lib/base'
	import AIChatModelSettings from '../copilot/chat/AIChatModelSettings.svelte'
	import FreeTierUsageIndicator from '../copilot/chat/FreeTierUsageIndicator.svelte'
	import HomeConnectDrawer from './HomeConnectDrawer.svelte'

	let value = $state('')
	let placeholder = $state('')
	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

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

	// Disabled because the user spent their free Windmill AI grant, not because AI was never
	// set up — the two look identical otherwise, and the "configure AI" copy would be a lie.
	let freeTierExhausted = $derived($copilotInfo.freeTier?.exhausted === true)

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

	const prompts = homeAIExamples.map((e) => e.prompt)

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
		<div
			class={disabled
				? 'transition-[filter] group-hover:blur-sm pointer-events-none select-none'
				: ''}
		>
			<p class="text-center font-regular text-3xl mb-4">Build with AI</p>
			<!-- anchors the send button / model settings to the input, not to the whole
			     block — the row below would otherwise push them down -->
			<div class="relative">
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
				<div class="absolute left-3 bottom-4 flex items-center gap-1.5 px-0.5">
					<AIChatModelSettings />
					<FreeTierUsageIndicator />
				</div>
			</div>

			<div class="flex items-center justify-between gap-2">
				<div class="flex flex-row flex-wrap items-center gap-1.5">
					{#each homeAIExamples as example (example.label)}
						<Button
							variant="default"
							unifiedSize="xs"
							btnClasses="!rounded-full !text-2xs !text-hint"
							onClick={() => (value = example.prompt)}
						>
							{example.label}
						</Button>
					{/each}
				</div>

				<div class="flex flex-row items-center gap-1">
					<Button
						variant="subtle"
						unifiedSize="xs"
						btnClasses="!text-2xs !text-hint"
						startIcon={{ icon: PlugZap }}
						onClick={() => homeConnectDrawer?.openDrawer?.()}
					>
						CLI / MCP
					</Button>
					{#if !$userStore?.operator && HOME_SHOW_HUB}
						<Button
							variant="subtle"
							unifiedSize="xs"
							btnClasses="!text-2xs !text-hint"
							startIcon={{ icon: Globe2 }}
							endIcon={{ icon: ExternalLink }}
							href={$hubBaseUrlStore}
							target="_blank"
						>
							Hub
						</Button>
					{/if}
				</div>
			</div>
		</div>
		{#if disabled}
			<div
				class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 rounded-md bg-surface/70 opacity-0 transition-opacity pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto"
			>
				<p class="text-sm text-secondary">
					{freeTierExhausted
						? 'You have used all of your free Windmill AI tokens'
						: 'No AI provider is configured'}
				</p>
				<Button
					unifiedSize="sm"
					variant="accent"
					startIcon={{ icon: freeTierExhausted ? KeyRound : Settings }}
					href="{base}/workspace_settings?tab=ai"
				>
					{freeTierExhausted ? 'Add your own API key' : 'Configure AI'}
				</Button>
			</div>
		{/if}
	</div>
</div>

<HomeConnectDrawer bind:this={homeConnectDrawer} />
