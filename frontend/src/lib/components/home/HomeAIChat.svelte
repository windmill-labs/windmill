<script lang="ts" module>
	import type HomeSessionStarterComponent from '../sessions/HomeSessionStarter.svelte'

	// The starter is a full AIChat, which reaches monaco, and the build refuses monaco in Home's
	// static imports. Kept once loaded, so a return to Home renders it in its first frame: the
	// AI Sessions → Home view transition snapshots that frame.
	let loadedStarter: typeof HomeSessionStarterComponent | undefined
	function loadStarter() {
		return import('../sessions/HomeSessionStarter.svelte').then((m) => (loadedStarter = m.default))
	}
</script>

<script lang="ts">
	import { ExternalLink, Globe2, PinOff, PlugZap } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { hubBaseUrlStore, roleCapabilities, workspaceStore } from '$lib/stores'
	import { homeDraft, sessionState } from '../sessions/sessionState.svelte'
	import { homeSidebarList } from '../sessions/sidebarList.svelte'
	import { currentWorkspaceRootId } from '../sessions/sessionScope.svelte'
	import { HOME_SHOW_HUB } from '$lib/consts'
	import { homeComposerPin, setHomeComposerRemoved } from './homeComposerPin.svelte'
	import { isRuleActive, protectionRulesState } from '$lib/workspaceProtectionRules.svelte'
	import { useReducedMotion } from '$lib/svelte5Utils.svelte'
	import { BROWSER } from 'esm-env'
	import HomeConnectDrawer from './HomeConnectDrawer.svelte'
	import HomeComposerSkeleton, {
		composerHeight,
		rememberComposerHeight
	} from './HomeComposerSkeleton.svelte'
	import { prefersSessionHandoff } from '../copilot/chat/global/gate'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import { prefetchItemCount } from '../copilot/chat/starter/starterPrompts.svelte'
	import { useWorkspaceAiHidden } from '../copilot/workspaceAiHidden.svelte'
	import { homeQuickAccessSettings } from './homeQuickAccessSettings.svelte'

	/** Bar width from which the two side trips stand in the bar rather than folding into the menu.
	 *  Home's trail is ~250px and the rest of its actions ~400, so this leaves room for both
	 *  labelled buttons without the trail starting to truncate. Below it they fold back. */
	const BAR_FITS_SIDE_TRIPS = 1024
	const sideTripsInBar = $derived(pageHeader.barWidth >= BAR_FITS_SIDE_TRIPS)
	const hubOffered = $derived($roleCapabilities.code && HOME_SHOW_HUB)

	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

	// Whether this reader wants the composer on the home page at all. Starting a session puts
	// it back (see pinHomeComposer).
	const removed = $derived(homeComposerPin.removed)

	// Removing puts the composer away rather than throwing it out, and the way back is one item in
	// the band's menu — so the ellipsis flashes as it appears, in a corner nobody is watching while
	// the composer leaves the page.
	//
	// A keyframe animation, not a transition: this mounts in the same flush that removes the
	// composer, so the frame that would paint it at rest is the frame it is already told to leave, and
	// one `requestAnimationFrame` is not reliably past that paint. The flash then goes straight to
	// invisible without ever being drawn. Mounted only while pulsing, so each removal replays it.
	const PULSE_MS = 1000
	const reducedMotion = useReducedMotion()
	let pulsing = $state(false)
	let pulseEndTimer: ReturnType<typeof setTimeout> | undefined

	function removeComposer() {
		// The `close` key keeps these events in the series they have always been counted in.
		logFeatureUsage('home', 'ai_hero_remove', { key: 'close' })
		setHomeComposerRemoved(true)
		if (reducedMotion.val) return
		clearTimeout(pulseEndTimer)
		pulsing = true
		pulseEndTimer = setTimeout(() => (pulsing = false), PULSE_MS)
	}

	// A workspace locked against direct deployment is run, not authored in, so its home page drops
	// the composer and the button to reopen it. This is about the workspace, not the caller: an
	// admin bypasses the lock outright, yet that does not make prod the place to start a session.
	// Unresolved rules read as unlocked, so the far commoner unlocked workspace never pops the
	// composer in mid-load.
	let runOnlyWorkspace = $derived(isRuleActive('DisableDirectDeployment'))

	// So the block waits for the rules before it mounts: mounted before them, a locked workspace
	// would show its composer until they arrived. Once shown for a family it stays: switching to a
	// fork resets the rules, and remounting would flash it.
	let rulesReady = $derived(
		protectionRulesState.rulesets !== undefined &&
			protectionRulesState.workspace === $workspaceStore
	)
	let shownForRoot = $state<string>()

	let HomeSessionStarter = $state(loadedStarter)
	if (!loadedStarter) void loadStarter().then((c) => (HomeSessionStarter = c))
	// The starter waits on the workspace's item count before it mounts; asked alongside its code
	// rather than once the code has loaded, it is no extra wait.
	if ($workspaceStore) prefetchItemCount($workspaceStore)

	// Whether Home's own workspace hides the assistant, read for that workspace rather than from
	// the shared copilot config, which still describes the last session's workspace for a moment
	// after leaving its page.
	const homeAiHidden = useWorkspaceAiHidden(() => $workspaceStore ?? undefined)

	// The composer hands off to /sessions, which refuses roles that cannot code, users opted out
	// of the sessions beta and a session's own preview frame — so hide it there (the prompt would
	// be silently dropped) while the AI-independent CLI/MCP row below stays.
	const inPreviewFrame = BROWSER && window.self !== window.top
	let showComposer = $derived(
		prefersSessionHandoff(!$roleCapabilities.code) &&
			!inPreviewFrame &&
			!runOnlyWorkspace &&
			!homeAiHidden.hidden
	)

	let hero = $derived(showComposer && !removed)
</script>

<div class="w-full {hero ? 'mt-2' : 'mb-1'}">
	<div class="relative group/composer">
		{#if hero}
			<!-- In the page's side padding, clear of the composer's own controls at its right end,
			     and centred on the composer's first row: the one-line box's middle, and where that
			     row stays once the box opens downward. Shown on hover; the container starts at the
			     composer's edge so the pointer reaches the unpin button without leaving the hovered
			     zone. Below `sm` there is no side padding to hold it, so unpinning is in the band's
			     menu instead. -->
			<div
				class="hidden sm:block absolute top-4 left-full pl-1 z-20 transition-opacity opacity-0 group-hover/composer:opacity-100 focus-within:opacity-100"
			>
				<CloseButton small noBg Icon={PinOff} title="Unpin from Home" onClick={removeComposer} />
			</div>
			<!-- The block is a session's chat, so it waits for the sessions it might reuse. Keyed on
			     the workspace family, not the workspace: another family needs its own draft, but a
			     fork shares its parent's, and remounting for it only flashes the block. Also keyed
			     on the asks for another draft (New session, a draft picked in the session list). -->
			{#if HomeSessionStarter && sessionState.hydrated && (rulesReady || shownForRoot === $currentWorkspaceRootId)}
				<!-- Going into the composer is going to the AI side, so the sidebar lists sessions,
				     where the one sent from here opens. Nothing may focus the composer unasked (Home's
				     search holds the page's first focus), or every visit would flip the list. -->
				<div class="contents" onfocusin={() => (homeSidebarList.val = 'sessions')}>
					{#key `${$currentWorkspaceRootId}:${homeDraft.requests}`}
						<HomeSessionStarter
							onShown={() => (shownForRoot = $currentWorkspaceRootId)}
							{placeholder}
							onSettled={(height) => rememberComposerHeight($workspaceStore ?? undefined, height)}
						/>
					{/key}
				</div>
			{:else}
				{@render placeholder()}
			{/if}
		{/if}
	</div>
</div>

{#snippet placeholder()}
	<HomeComposerSkeleton height={composerHeight($workspaceStore ?? undefined)} />
{/snippet}

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
		     unpin button removes it, so an ellipsis holding a second way to do that is a button whose only
		     item is a duplicate. -->
		{#if !sideTripsInBar || (showComposer && removed) || homeQuickAccessSettings.hidden}
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
				<!-- The same flash for hiding the favorites & recent section. It rests at opacity 0, so
				     it can stay mounted; the key replays it on each hide. -->
				{#if homeQuickAccessSettings.hideCount > 0 && !reducedMotion.val}
					{#key homeQuickAccessSettings.hideCount}
						<span
							aria-hidden="true"
							class="pointer-events-none absolute -inset-1 rounded-lg bg-blue-500/30 opacity-0 animate-fade-out"
						></span>
					{/key}
				{/if}
				<DropdownV2
					placement="bottom-end"
					size="sm"
					customWidth={240}
					items={[
						// Every phone gets the menu, and there the composer has no room for its own unpin
						// button. The menu can also hold the section's pin, so the label names the session.
						...(hero ? [{ displayName: 'Unpin session from Home', action: removeComposer }] : []),
						...(showComposer && removed
							? [
									{
										displayName: 'Pin session hero to homepage',
										action: () => {
											logFeatureUsage('home', 'ai_hero_pin')
											setHomeComposerRemoved(false)
										}
									}
								]
							: []),
						...(homeQuickAccessSettings.hidden
							? [
									{
										displayName: 'Pin favorites & recent to homepage',
										action: () => homeQuickAccessSettings.setHidden(false)
									}
								]
							: []),
						...(sideTripsInBar
							? []
							: [
									{
										displayName: 'CLI / MCP',
										action: () => homeConnectDrawer?.openDrawer?.()
									},
									...(hubOffered
										? [
												{
													displayName: 'Hub',
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
