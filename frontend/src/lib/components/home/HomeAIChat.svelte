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
	import { ExternalLink, Globe2, PlugZap } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { hubBaseUrlStore, roleCapabilities, workspaceStore } from '$lib/stores'
	import { homeDraft, sessionState } from '../sessions/sessionState.svelte'
	import { currentWorkspaceRootId } from '../sessions/sessionScope.svelte'
	import { HOME_SHOW_HUB } from '$lib/consts'
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
	import { prefetchComposerSignals } from '../copilot/chat/starter/starterPrompts.svelte'
	import { useWorkspaceAiHidden } from '../copilot/workspaceAiHidden.svelte'
	import { homeQuickAccessSettings } from './homeQuickAccessSettings.svelte'

	/** Bar width from which the two side trips stand in the bar rather than folding into the menu.
	 *  Home's trail is ~250px and the rest of its actions ~400, so this leaves room for both
	 *  labelled buttons without the trail starting to truncate. Below it they fold back. */
	const BAR_FITS_SIDE_TRIPS = 1024
	const sideTripsInBar = $derived(pageHeader.barWidth >= BAR_FITS_SIDE_TRIPS)
	const hubOffered = $derived($roleCapabilities.code && HOME_SHOW_HUB)

	let homeConnectDrawer: HomeConnectDrawer | undefined = $state(undefined)

	const reducedMotion = useReducedMotion()

	let HomeSessionStarter = $state(loadedStarter)
	if (!loadedStarter) void loadStarter().then((c) => (HomeSessionStarter = c))
	// The starter waits on the workspace's item count before it mounts, and its suggestions on
	// the other signals; asked alongside its code rather than once the code has loaded, they
	// are no extra wait.
	if ($workspaceStore) prefetchComposerSignals($workspaceStore)

	// Whether Home's own workspace hides the assistant, read for that workspace rather than from
	// the shared copilot config, which still describes the last session's workspace for a moment
	// after leaving its page.
	const homeAiHidden = useWorkspaceAiHidden(() => $workspaceStore ?? undefined)

	// The composer hands off to /sessions, which refuses roles that cannot code, users opted out
	// of the sessions beta and a session's own preview frame — so hide it there (the prompt would
	// be silently dropped) while the AI-independent CLI/MCP row below stays.
	const inPreviewFrame = BROWSER && window.self !== window.top
	let showComposer = $derived(
		prefersSessionHandoff(!$roleCapabilities.code) && !inPreviewFrame && !homeAiHidden.hidden
	)

</script>

<div class="w-full {showComposer ? 'mt-2' : 'mb-1'}">
	<div class="relative group/composer">
		{#if showComposer}
			<!-- The block is a session's chat, so it waits for the sessions it might reuse. Keyed on
			     the workspace family, not the workspace: another family needs its own draft, but a
			     fork shares its parent's, and remounting for it only flashes the block. Also keyed
			     on the asks for another draft (New session, a draft picked in the session list). -->
			{#if HomeSessionStarter && sessionState.hydrated}
				{#key `${$currentWorkspaceRootId}:${homeDraft.requests}`}
					<HomeSessionStarter
						{placeholder}
						onSettled={(height) => rememberComposerHeight($workspaceStore ?? undefined, height)}
					/>
				{/key}
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

<!-- Everything the hero offered beside the composer lives in the band's menu: the connect helper
     and the hub. -->
<!-- Last in the bar whatever else the page registers: a menu of side trips belongs after the
     buttons that act on what is on screen. -->
<PageHeaderContent actions={homeMenu} actionsOrder={100} />

{#snippet homeMenu()}
	<!-- The band's own menu: the connect helper, the hub, and the way back to favorites & recent
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
		<!-- Nothing left to hold once the side trips stand in the bar: an ellipsis that opens an
		     empty menu is a button that does nothing. -->
		{#if !sideTripsInBar || homeQuickAccessSettings.hidden}
			<span class="relative inline-flex">
				<!-- Hiding favorites & recent puts its way back in this menu, so the ellipsis flashes on
				     each hide. It rests at opacity 0, so it can stay mounted; the key replays it. -->
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
