<script lang="ts">
	import { untrack, getContext, type Snippet } from 'svelte'
	import { fade } from 'svelte/transition'
	import { cubicOut } from 'svelte/easing'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Search, Maximize2, Minimize2, PanelLeftClose, PanelLeftOpen } from 'lucide-svelte'
	import { page } from '$app/state'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { getModifierKey } from '$lib/utils'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { Button } from '$lib/components/common'
	import { Menubar } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import SettingsMenu from '$lib/components/sidebar/SettingsMenu.svelte'
	import SidebarContent from '$lib/components/sidebar/SidebarContent.svelte'
	import FavoriteMenu, { favoriteManager } from '$lib/components/sidebar/FavoriteMenu.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import SessionPicker from './SessionPicker.svelte'
	import SessionWrapper from './SessionWrapper.svelte'
	import {
		createSession,
		sessionState,
		setSessionTarget,
		syncWorkspaceTo,
		type SessionTarget
	} from './sessionState.svelte'

	let { children }: { children?: Snippet } = $props()

	const openSearch = getContext<(text?: string) => void>('openSearchWithPrefilledText')

	// Collapse the session rail into a popover launcher (sticky).
	const railCollapsed = useLocalStorageValue('wm_session_rail_collapsed', false, 'boolean')
	let railMenuOpen = $state(false)

	// Browse mode: clicking a workspace in the tree shows just the panel preview at
	// its home — no chat — so workspaces can be explored without an AI session.
	// Clicking a session leaves browse mode and brings the chat back.
	let browseMode = $state(false)
	// The explicitly-selected workspace (highlighted in the tree). Set on click so
	// the selection is reliable rather than derived from the URL.
	let selectedWorkspaceId = $state<string | undefined>(undefined)

	// Full screen: hide the whole session chrome (rail + chat) so the embedded page
	// fills the viewport.
	let panelFullscreen = $state(false)

	function browseWorkspace(workspaceId: string) {
		browseMode = true
		selectedWorkspaceId = workspaceId
		syncWorkspaceTo(workspaceId)
		goto('/')
	}

	function selectSession() {
		browseMode = false
		selectedWorkspaceId = undefined
	}

	// Edge-to-edge panel (no padding/border): full screen, or while browsing a
	// workspace so its preview fills the panel area cleanly.
	const edgeToEdge = $derived(panelFullscreen || browseMode)

	// Resolve the selected session, but only if its record actually exists.
	// A stale currentSessionId can outlive its session (e.g. a transient draft
	// dropped on reload), and mounting the wrapper on a missing id renders a
	// "Session not found" stub.
	const activeId = $derived(
		sessionState.currentSessionId &&
			sessionState.sessions.some((s) => s.id === sessionState.currentSessionId)
			? sessionState.currentSessionId
			: undefined
	)

	const chatVisible = $derived(!!activeId && !browseMode && !panelFullscreen)

	// No valid session selected → spin up (or reuse) the transient draft so the
	// center shows a ready-to-type composer. Sending the first message commits
	// it; until then nothing is persisted.
	$effect(() => {
		if (!activeId) {
			untrack(() => createSession())
		}
	})

	// Parse the embedded panel's route back into a session editor target.
	function parseTarget(pathname: string): SessionTarget | undefined {
		const p = pathname.startsWith(base) ? pathname.slice(base.length) : pathname
		const m = /^\/(scripts|flows|apps_raw)\/(?:edit|add|get)\/(.+)$/.exec(p)
		if (!m) return undefined
		const kind = m[1] === 'scripts' ? 'script' : m[1] === 'flows' ? 'flow' : 'raw_app'
		return { kind, path: decodeURIComponent(m[2]) }
	}

	// "Target follows the panel": as the embedded Windmill route changes, retarget
	// the active session at whatever item it now shows. Only committed sessions —
	// retargeting a transient would persist it before its first message. Skip in
	// browse mode (no chat) so exploring doesn't repoint a session.
	$effect(() => {
		const id = activeId
		const t = parseTarget(page.url.pathname)
		if (!id || !t || browseMode) return
		const s = sessionState.sessions.find((x) => x.id === id)
		if (!s || s.transient) return
		if (s.target?.kind === t.kind && s.target?.path === t.path) return
		untrack(() => setSessionTarget(id, t))
	})
</script>

<div class="h-screen w-screen flex flex-row overflow-hidden relative">
	{#snippet railBody()}
		<div class="flex-1 min-h-0 overflow-y-auto">
			<SessionPicker
				isCollapsed={false}
				collapsible={false}
				workspaceTree
				browsedWorkspaceId={selectedWorkspaceId}
				onBrowseWorkspace={browseWorkspace}
				onSelectSession={selectSession}
			/>
		</div>
		<!-- Account / instance actions gathered under one "Settings" dropdown. -->
		<div class="shrink-0 border-t border-light p-2">
			<SettingsMenu />
		</div>
	{/snippet}

	{#if !panelFullscreen}
		{#if !railCollapsed.val}
			<!-- Rail: workspace tree + sessions. Replaces the global nav. -->
			<div class="w-56 shrink-0 flex flex-col border-r border-light bg-surface-secondary min-h-0">
				<div class="flex items-center gap-2 px-3 h-12 border-b border-light shrink-0">
					<button
						type="button"
						onclick={() => goto('/')}
						title="Home"
						aria-label="Home"
						class="shrink-0 flex items-center"
					>
						<WindmillIcon height="20px" width="20px" />
					</button>
					<span class="text-sm font-semibold text-emphasis">Windmill</span>
					<button
						type="button"
						onclick={() => (railCollapsed.val = true)}
						title="Collapse sidebar"
						aria-label="Collapse sidebar"
						class="ml-auto shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
					>
						<PanelLeftClose size={16} />
					</button>
				</div>
				{@render railBody()}
			</div>
		{:else}
			<!-- Collapsed: no rail/bar — a floating launcher that reveals the session
			     tree on hover. The popover sits inside the hover container (with a
			     transparent bridge) so moving onto it doesn't close the menu. -->
			<div
				class="absolute top-0.5 left-1.5 z-50"
				role="presentation"
				onmouseenter={() => (railMenuOpen = true)}
				onmouseleave={() => (railMenuOpen = false)}
			>
				<Button
					variant="subtle"
					unifiedSize="sm"
					iconOnly
					startIcon={{ icon: PanelLeftOpen }}
					title="Open sessions"
				/>
				{#if railMenuOpen}
					<div
						class="absolute top-full left-0 pt-1 w-64"
						transition:fade={{ duration: 130, easing: cubicOut }}
					>
						<div
							class="max-h-[calc(100vh-3rem)] flex flex-col bg-surface border border-light rounded-md shadow-lg overflow-hidden"
						>
							<div class="flex items-center gap-2 px-3 h-10 border-b border-light shrink-0">
								<span class="text-sm font-semibold text-emphasis">Sessions</span>
								<button
									type="button"
									onclick={() => (railCollapsed.val = false)}
									title="Pin sidebar"
									aria-label="Pin sidebar"
									class="ml-auto shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
								>
									<PanelLeftClose size={16} />
								</button>
							</div>
							{@render railBody()}
						</div>
					</div>
				{/if}
			</div>
		{/if}
	{/if}

	<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
		{#if chatVisible && activeId}
			<!-- Chat pane (only when a session is active and not browsing). -->
			<Pane size={38} minSize={24} class="flex flex-col min-h-0">
				{#key activeId}
					<SessionWrapper sessionId={activeId} hideEditor headerInset={railCollapsed.val} />
				{/key}
			</Pane>
		{/if}

		<!-- Panel = the live Windmill page, framed like the session editor pane. -->
		<Pane minSize={30} class="flex flex-col min-h-0">
			<div class="flex-1 min-h-0 flex flex-col {edgeToEdge ? 'p-0' : 'p-2 pl-0'}">
				<div
					class="flex flex-col flex-1 min-h-0 overflow-hidden relative bg-surface {edgeToEdge
						? ''
						: 'rounded-md border border-light'}"
				>
					<!-- Viewing breadcrumb: full-width panel header, on top of the nav sidebar. -->
					<div
						class="pr-3 h-8 flex items-center gap-2 border-b border-light shrink-0 text-xs text-secondary {railCollapsed.val &&
						!chatVisible
							? 'pl-11'
							: 'pl-3'}"
					>
						<span class="text-tertiary">Viewing</span>
						<span class="font-mono truncate">{page.url.pathname}</span>
						{#if chatVisible || panelFullscreen}
							<!-- Maximizing only makes sense when there's a chat to hide (or to
							     exit full screen) — not while browsing a workspace. -->
							<button
								type="button"
								onclick={() => (panelFullscreen = !panelFullscreen)}
								title={panelFullscreen ? 'Exit full screen' : 'Full screen'}
								aria-label={panelFullscreen ? 'Exit full screen' : 'Full screen'}
								class="ml-auto shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
							>
								{#if panelFullscreen}
									<Minimize2 size={14} />
								{:else}
									<Maximize2 size={14} />
								{/if}
							</button>
						{/if}
					</div>
					<div class="flex flex-row flex-1 min-h-0">
						<!-- Temporary nav sidebar so the panel stays navigable. Mirrors the global
						     sidebar minus the workspace picker (which would let you switch workspace
						     out from under the active session). -->
						<div
							class="w-12 shrink-0 flex flex-col border-r border-light bg-surface min-h-0 overflow-y-auto"
						>
							<div class="px-2 py-2 border-b border-light dark:border-gray-700 flex flex-col gap-1">
								<Menubar class="flex flex-col gap-1">
									{#snippet children({ createMenu })}
										<FavoriteMenu
											{createMenu}
											favoriteLinks={favoriteManager.current}
											isCollapsed
										/>
									{/snippet}
								</Menubar>
								<MenuButton
									stopPropagationOnClick={true}
									on:click={() => openSearch?.()}
									isCollapsed
									icon={Search}
									label="Search"
									class="!text-xs"
									shortcut={`${getModifierKey()}k`}
								/>
							</div>
							<SidebarContent
								isCollapsed
								showSecondary={false}
								numUnacknowledgedCriticalAlerts={0}
							/>
						</div>

						<div id="content" class="flex-1 min-h-0 flex flex-col overflow-hidden">
							{@render children?.()}
						</div>
					</div>
				</div>
			</div></Pane
		>
	</Splitpanes>
</div>

<style>
	/* Invisible-but-draggable splitter between the chat and the panel. */
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
