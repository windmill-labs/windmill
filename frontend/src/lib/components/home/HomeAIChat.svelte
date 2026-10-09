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
	import CloseButton from '../common/CloseButton.svelte'
	import { startSessionWithPrompt } from '../sessions/sessionSwitch.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import {
		aiUserDisabled,
		hubBaseUrlStore,
		userStore,
		userWorkspaces,
		workspaceStore
	} from '$lib/stores'
	import ActingOnPicker from '../sessions/ActingOnPicker.svelte'
	import { defaultSessionWorkspace, type PendingFork } from '../sessions/sessionState.svelte'
	import AutonomyModePicker from '../copilot/chat/AutonomyModePicker.svelte'
	import {
		AIAutonomyMode,
		getPersistedAutonomyMode,
		AIMode,
		supportsAutoAcceptEdits,
		supportsAutoAcceptToolConfirmations,
		supportsPlanMode
	} from '../copilot/chat/AIChatManager.svelte'
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
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import BuildWithAIHeading from '../copilot/chat/starter/BuildWithAIHeading.svelte'
	import StarterPromptChips from '../copilot/chat/starter/StarterPromptChips.svelte'
	import {
		PLACEHOLDER_FADE_CLASS,
		RotatingPlaceholder,
		StarterPrompts
	} from '../copilot/chat/starter/starterPrompts.svelte'

	const COLLAPSED_SETTING = 'home-ai-composer-collapsed'

	/** Bar width from which the two side trips stand in the bar rather than folding into the menu.
	 *  Home's trail is ~250px and the rest of its actions ~400, so this leaves room for both
	 *  labelled buttons without the trail starting to truncate. Below it they fold back. */
	const BAR_FITS_SIDE_TRIPS = 1024
	const sideTripsInBar = $derived(pageHeader.barWidth >= BAR_FITS_SIDE_TRIPS)
	const hubOffered = $derived(!$userStore?.operator && HOME_SHOW_HUB)

	let value = $state('')
	const starterPrompts = new StarterPrompts()
	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

	// How much of the home page this reader wants the composer to take, so it lives per browser
	// rather than per workspace or account.
	let collapsed = $state(BROWSER && getLocalSetting(COLLAPSED_SETTING) === 'true')
	function setCollapsed(next: boolean) {
		collapsed = next
		storeLocalSetting(COLLAPSED_SETTING, next ? 'true' : undefined)
	}

	// Removing puts the composer away rather than throwing it out, and the way back is one item in
	// the band's menu — so the ellipsis flashes as it appears, in a corner nobody is watching while
	// the hero collapses in the middle of the page.
	//
	// A keyframe animation, not a transition: this mounts in the same flush that collapses the
	// hero, so the frame that would paint it at rest is the frame it is already told to leave, and
	// one `requestAnimationFrame` is not reliably past that paint. The flash then goes straight to
	// invisible without ever being drawn. Mounted only while pulsing, so each removal replays it.
	const PULSE_MS = 1000
	const reducedMotion = useReducedMotion()
	let pulsing = $state(false)
	let pulseEndTimer: ReturnType<typeof setTimeout> | undefined

	function removeComposer() {
		setCollapsed(true)
		if (reducedMotion.val) return
		clearTimeout(pulseEndTimer)
		pulsing = true
		pulseEndTimer = setTimeout(() => (pulsing = false), PULSE_MS)
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

	// The composer hands off to /sessions, which refuses operators and users opted out of the
	// sessions beta — so hide it from them (the prompt would be silently dropped) while the
	// AI-independent CLI/MCP row below stays.
	let showComposer = $derived(
		prefersSessionHandoff($userStore?.operator) &&
			!runOnlyWorkspace &&
			!$copilotInfo.workspaceDisabled
	)

	// The hero's margins and centered column are for the full block. The lone button row left
	// by a collapsed, opted-out, run-only or hidden-assistant view is a hint line and should
	// cost the page almost nothing: no top margin, and the content column's full width so it
	// hugs the right edge instead of floating centered in empty space.
	let hero = $derived(showComposer && !collapsed)
	let outerSpacing = $derived(hero ? 'mt-20 mb-16' : 'mt-0 mb-1')

	// The session's own pre-send controls, held here until the hand-off creates the session.
	// A pick is kept with the workspace it was made in, so switching workspace drops it
	// rather than carrying a workspace from another family into the new session.
	let actingOnPick = $state<{ in: string; workspaceId: string; fork?: PendingFork }>()
	const pickHere = $derived(actingOnPick?.in === $workspaceStore ? actingOnPick : undefined)
	// defaultSessionWorkspace reads these stores with `get`, so name them to re-run on change.
	const actingOnId = $derived.by(() => {
		void $userWorkspaces
		void $workspaceStore
		return pickHere?.workspaceId ?? defaultSessionWorkspace()
	})
	function pickActingOn(workspaceId: string, fork?: PendingFork) {
		if ($workspaceStore) actingOnPick = { in: $workspaceStore, workspaceId, fork }
	}

	// Only a mode picked here is handed off: the persisted one is what the session would
	// start with anyway, and it can read stale before the user's email resolves.
	let pickedAutonomyMode = $state<AIAutonomyMode>()
	// The persisted mode is keyed by the user's email, so re-read it once that resolves.
	const autonomyMode = $derived.by(() => {
		void $userStore?.email
		return pickedAutonomyMode ?? getPersistedAutonomyMode()
	})
	const sessionAutonomyAvailability = {
		autoAcceptEditsAvailable: supportsAutoAcceptEdits(AIMode.GLOBAL),
		autoAcceptToolConfirmationsAvailable: supportsAutoAcceptToolConfirmations(AIMode.GLOBAL),
		planModeAvailable: supportsPlanMode(AIMode.GLOBAL)
	}

	let starting = $state(false)
	async function start() {
		if (!canSend || starting || !value.trim()) return
		starting = true
		try {
			await startSessionWithPrompt(value, {
				autoSend: true,
				actingOn: pickHere,
				autonomyMode: pickedAutonomyMode
			})
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

	const placeholder = new RotatingPlaceholder(
		() => starterPrompts.list.map((e) => e.prompt),
		() => showComposer && !collapsed
	)
</script>

<div class="w-full flex justify-center {outerSpacing}">
	<div class="{hero ? 'max-w-[40rem]' : ''} grow relative group">
		{#if showComposer && !collapsed}
			{#if !disabled}
				<!-- The one dismiss control while the composer is usable; the overlay below carries its
				     own once it takes over, so the two never show at the same time. -->
				<div class="absolute right-0 top-0 z-20">
					<CloseButton small noBg title="Remove session chat" onClick={removeComposer} />
				</div>
			{/if}
			<BuildWithAIHeading />
			<ActingOnPicker
				selectedId={actingOnId}
				pendingFork={pickHere?.fork}
				onPick={(id) => pickActingOn(id)}
				onCreateFork={(fork) => pickActingOn(fork.parent_workspace_id, fork)}
			/>
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
						<!-- The 1px margin is the textarea's border, so the text sits where typing starts. -->
						<span
							aria-hidden="true"
							class="pointer-events-none absolute inset-x-4 top-3 m-px text-xs text-hint {PLACEHOLDER_FADE_CLASS} {placeholder.visible
								? 'opacity-100'
								: 'opacity-0'}"
						>
							{placeholder.text}
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
						<AutonomyModePicker
							mode={autonomyMode}
							availability={sessionAutonomyAvailability}
							onChange={(mode) => (pickedAutonomyMode = mode)}
						/>
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
							<Button unifiedSize="sm" variant="default" onClick={removeComposer}>Remove</Button>
						</div>
					</div>
				{/if}
			</div>
		{/if}

		<div class="flex items-center justify-between gap-2 pt-2">
			{#if showComposer && !collapsed}
				<StarterPromptChips prompts={starterPrompts.list} onPick={(p) => (value = p)} />
			{/if}
		</div>
	</div>
</div>

<HomeConnectDrawer bind:this={homeConnectDrawer} />

<!-- Everything the hero offered beside the composer lives in the band's menu: the connect helper,
     the hub, and the way back to the composer once it has been put away. -->
<!-- Last in the bar whatever else the page registers: a menu of side trips belongs after the
     buttons that act on what is on screen. -->
<PageHeaderContent actions={homeMenu} actionsOrder={100} />

{#snippet homeMenu()}
	<!-- The band's own menu: the connect helper, the hub, and whether the composer is on this page
	     — all of them preferences or side trips, none of them the page's work. -->
	<span class="flex items-center gap-1">
		<!-- Wide enough, and the two side trips are buttons of their own: a menu is where a thing
		     goes when the bar cannot hold it, not where it belongs. They fold back below the width
		     above. -->
		{#if sideTripsInBar}
			<!-- The same quiet shape the hero gave them before the bar took them: xs, hint-coloured,
			     and the hub keeps its outbound mark. They sit beside the page's own buttons, and a
			     side trip should not read as loudly as the thing the page is for. -->
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses="!text-2xs !text-hint"
				startIcon={{ icon: PlugZap }}
				onClick={() => homeConnectDrawer?.openDrawer?.()}
			>
				CLI / MCP
			</Button>
			{#if hubOffered}
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
		{/if}
		<!-- Nothing left to hold once the side trips stand in the bar and the composer is on the
		     page: an ellipsis that opens an empty menu is a button that does nothing. -->
		<!-- No menu while the hero is on the page and its side trips are in the bar: the hero's own
		     cross removes it, so an ellipsis holding a second way to do that is a button whose only
		     item is a duplicate. -->
		{#if !sideTripsInBar || (showComposer && collapsed)}
			<!-- The flash rides the ellipsis itself, not the row: with the side trips beside it the
			     row is three controls wide, and marking all three names none of them. -->
			<span class="relative inline-flex">
				{#if pulsing}
					<!-- `opacity-0` at rest: the keyframes carry no fill mode, so at the end of the
					     animation the element returns to its own opacity. At 1 that is the flash coming
					     back at full strength until the unmount timer catches up. -->
					<span
						aria-hidden="true"
						class="pointer-events-none absolute -inset-1 rounded-lg bg-blue-500/30 opacity-0 animate-fade-out"
					></span>
				{/if}
				<DropdownV2
					placement="bottom-end"
					size="sm"
					items={[
						...(showComposer && collapsed
							? [
									{
										displayName: 'Pin session hero to homepage',
										icon: WandSparkles,
										action: () => setCollapsed(false)
									}
								]
							: []),
						...(sideTripsInBar
							? []
							: [
									{
										displayName: 'CLI / MCP',
										icon: PlugZap,
										action: () => homeConnectDrawer?.openDrawer?.()
									},
									...(hubOffered
										? [
												{
													displayName: 'Hub',
													icon: Globe2,
													href: $hubBaseUrlStore,
													hrefTarget: '_blank' as const
												}
											]
										: [])
								])
					]}
				/>
			</span>
		{/if}
	</span>
{/snippet}
