<script lang="ts" module>
	/** Example prompts: the short `label` is shown as a clickable tag under the chat,
	 * the `prompt` is what rotates through the placeholder / gets dropped into the input. */
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
	import {
		ArrowUp,
		ExternalLink,
		Globe2,
		KeyRound,
		PlugZap,
		Settings,
		WandSparkles
	} from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { Badge } from '../common'
	import CloseButton from '../common/CloseButton.svelte'
	import { startSessionWithPrompt } from '../sessions/sessionSwitch.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { aiUserDisabled, hubBaseUrlStore, userStore, workspaceStore } from '$lib/stores'
	import { HOME_SHOW_HUB } from '$lib/consts'
	import { base } from '$lib/base'
	import { getLocalSetting, storeLocalSetting } from '$lib/utils'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { BROWSER } from 'esm-env'
	import AIChatModelSettings from '../copilot/chat/AIChatModelSettings.svelte'
	import HomeConnectDrawer from './HomeConnectDrawer.svelte'
	import { USER_SETTINGS_HASH } from '../sidebar/settings'
	import { prefersSessionHandoff } from '../copilot/chat/global/gate'

	const COLLAPSED_SETTING = 'home-ai-composer-collapsed'

	let value = $state('')
	let placeholder = $state(homeAIExamples[0].prompt)
	let placeholderVisible = $state(true)
	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

	// How much of the home page this reader wants the composer to take, so it lives per browser
	// rather than per workspace or account.
	let collapsed = $state(BROWSER && getLocalSetting(COLLAPSED_SETTING) === 'true')
	function setCollapsed(next: boolean) {
		collapsed = next
		storeLocalSetting(COLLAPSED_SETTING, next ? 'true' : undefined)
	}

	// In global-AI mode the layout's chat panel is disabled and never loads the copilot
	// config, so the home chat loads it for the current workspace itself.
	$effect(() => {
		if ($workspaceStore) {
			loadCopilot($workspaceStore)
		}
	})

	// Whether the copilot config has actually loaded for the current workspace.
	let configLoaded = $derived($copilotWorkspace === $workspaceStore)
	// No usable model (no provider configured, or AI disabled): the input is blurred and an overlay
	// explains why and links to the fix. Gate on `configLoaded` so the initial (unloaded) state
	// doesn't flash the overlay while a provider is in fact configured.
	let disabled = $derived(configLoaded && !$copilotInfo.enabled)
	// Submission is stricter than the overlay: block it until the config is loaded AND
	// enabled. Submitting during the unknown-config window hands the prompt to a session
	// that only sends once `copilotInfo.enabled` flips true — on an unconfigured/disabled
	// workspace that never happens and the prompt is silently lost.
	let canSend = $derived(configLoaded && $copilotInfo.enabled)

	// The input alone: what the overlay covers and the one part a missing provider makes unusable.
	let blurClass = $derived(disabled ? 'blur-sm pointer-events-none select-none' : '')

	// Disabled because the user spent their free Windmill AI grant, not because AI was never
	// set up — the two look identical otherwise, and the "configure AI" copy would be a lie.
	let freeTierExhausted = $derived($copilotInfo.freeTier?.exhausted === true)

	// A workspace locked against direct deployment is run, not authored in, so its home page drops
	// the composer and the button to reopen it. This is about the workspace, not the caller:
	// `createSession` would steer a prompt into the paired dev workspace and an admin bypasses the
	// lock outright, yet neither makes prod the place to start one. Unresolved rules read as
	// unlocked, so the far commoner unlocked workspace never pops the composer in mid-load.
	let runOnlyWorkspace = $derived(isRuleActive('DisableDirectDeployment'))

	// The composer hands off to /sessions, which refuses operators — so hide it from them (the
	// prompt would be silently dropped) while the AI-independent CLI/MCP row below stays.
	let showComposer = $derived(
		prefersSessionHandoff($userStore?.operator) &&
			!runOnlyWorkspace &&
			!$copilotInfo.workspaceDisabled
	)

	// The hero's margins and centered column are for the full block. The lone button row left
	// by a collapsed, operator, run-only or hidden-assistant view is a hint line and should
	// cost the page almost nothing: no top margin, and the content column's full width so it
	// hugs the right edge instead of floating centered in empty space.
	let hero = $derived(showComposer && !collapsed)
	let outerSpacing = $derived(hero ? 'mt-20 mb-16' : 'mt-0 mb-1')

	let starting = $state(false)
	async function start() {
		if (!canSend || starting || !value.trim()) return
		starting = true
		try {
			await startSessionWithPrompt(value, { autoSend: true })
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

	const CYCLE_MS = 7_000
	// Must match the `duration-*` class on the placeholder overlay: the swap happens once the
	// fade-out has finished, and Tailwind only emits classes it finds written out in full.
	const FADE_MS = 600

	// Rotate the example prompt every CYCLE_MS: fade the placeholder out, swap it, fade it back in.
	// Only while the composer is shown — otherwise (operators) it would loop forever driving an
	// unrendered input — and not under reduced motion, where the first prompt simply stays put.
	// The index lives outside the effect so re-showing the composer resumes the rotation from the
	// prompt currently displayed rather than restarting it.
	const reducedMotion = useReducedMotion()
	let promptIndex = 0
	$effect(() => {
		if (!showComposer || collapsed || reducedMotion.val) return
		let timer: ReturnType<typeof setTimeout>

		function next() {
			placeholderVisible = false
			timer = setTimeout(() => {
				promptIndex = (promptIndex + 1) % prompts.length
				placeholder = prompts[promptIndex]
				placeholderVisible = true
				timer = setTimeout(next, CYCLE_MS)
			}, FADE_MS)
		}

		timer = setTimeout(next, CYCLE_MS)
		return () => {
			clearTimeout(timer)
			// Torn down mid-fade, the input would otherwise remount with an invisible placeholder.
			placeholderVisible = true
		}
	})
</script>

<div class="w-full flex justify-center {outerSpacing}">
	<div class="{hero ? 'max-w-[40rem]' : ''} grow relative group">
		{#if showComposer && !collapsed}
			{#if !disabled}
				<!-- The one dismiss control while the composer is usable; the overlay below carries its
				     own once it takes over, so the two never show at the same time. -->
				<div class="absolute right-0 top-0 z-20">
					<CloseButton small noBg title="Hide Build with AI" onClick={() => setCollapsed(true)} />
				</div>
			{/if}
			<div class="flex items-center justify-center gap-2 mb-4">
				<p class="text-center font-regular text-3xl">Build with AI</p>
				<Badge color="blue" small>Beta</Badge>
			</div>
			<!-- Anchors the send button / model settings to the input, not to the whole block — the row
			     below would otherwise push them down. The inner wrapper stays `relative` in both
			     states: `blur-sm` is a filter, which makes an element the containing block for its
			     absolutely positioned children, so those two would shift when the blur turns on. -->
			<div class="relative">
				<div class="relative {blurClass}" inert={disabled}>
					<TextInput
						bind:value
						class="resize-none px-4 py-3 pb-9 shadow-sm border-accent"
						underlyingInputEl="textarea"
						inputProps={{
							rows: 4,
							'aria-label': 'Describe what you want to build',
							onkeydown: onKeydown
						}}
					/>
					{#if !value}
						<!-- Drawn over the textarea instead of set as its `placeholder`: WebKit and Gecko do
						     not run transitions on `::placeholder`, so the fade would be a hard cut there.
						     The 1px margin is the textarea's border, so the text sits where typing starts. -->
						<span
							aria-hidden="true"
							class="pointer-events-none absolute inset-x-4 top-3 m-px text-xs text-hint transition-opacity duration-[600ms] ease-in-out {placeholderVisible
								? 'opacity-100'
								: 'opacity-0'}"
						>
							{placeholder}
						</span>
					{/if}
					<Button
						endIcon={starting ? {} : { icon: ArrowUp }}
						wrapperClasses="absolute right-2 bottom-3.5"
						variant={value.trim() ? 'accent' : 'subtle'}
						iconOnly
						loading={starting}
						disabled={!value.trim() || starting || !canSend}
						onclick={start}
					></Button>
					<div class="absolute left-3 bottom-4 flex items-center gap-1.5 px-0.5">
						<AIChatModelSettings />
					</div>
				</div>
				{#if disabled}
					<!-- Covers the input alone: the title, the example prompts and the CLI/MCP row are all
					     still legible and usable without a provider. Static, not hover-gated, so keyboard
					     and touch users see it too. -->
					<div
						class="absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 rounded-md bg-surface/70"
					>
						<p class="text-sm text-secondary">
							{#if $aiUserDisabled}
								Windmill AI is disabled in your account settings
							{:else if freeTierExhausted}
								You have used all of your free Windmill AI tokens
							{:else}
								No AI provider is configured
							{/if}
						</p>
						<div class="flex items-center gap-2">
							{#if $aiUserDisabled}
								<!-- The fix lives in account settings (a hash-opened drawer, not a route), so link
								     the hash the sidebar's Account menu uses rather than the workspace AI settings. -->
								<Button
									unifiedSize="sm"
									variant="accent"
									startIcon={{ icon: Settings }}
									href={USER_SETTINGS_HASH}
								>
									Open account settings
								</Button>
							{:else}
								<Button
									unifiedSize="sm"
									variant="accent"
									startIcon={{ icon: freeTierExhausted ? KeyRound : Settings }}
									href="{base}/workspace_settings?tab=ai"
								>
									{freeTierExhausted ? 'Add your own API key' : 'Configure AI'}
								</Button>
							{/if}
							<Button unifiedSize="sm" variant="default" onClick={() => setCollapsed(true)}>
								Hide
							</Button>
						</div>
					</div>
				{/if}
			</div>
		{/if}

		<div class="flex items-center justify-between gap-2 pt-2">
			{#if showComposer && !collapsed}
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
			{:else if showComposer}
				<!-- All that is left of the composer once dismissed: sits with the CLI/MCP row so the
				     collapsed home page is one quiet line. -->
				<Button
					variant="subtle"
					unifiedSize="xs"
					btnClasses="!text-2xs !text-hint"
					startIcon={{ icon: WandSparkles }}
					onClick={() => setCollapsed(false)}
				>
					Build with AI
				</Button>
			{:else}
				<div></div>
			{/if}

			<!-- Not AI-related, so shown even to operators / when the composer is hidden. -->
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
</div>

<HomeConnectDrawer bind:this={homeConnectDrawer} />
