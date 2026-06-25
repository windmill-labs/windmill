<script lang="ts">
	import { untrack, getContext, onMount, type Snippet } from 'svelte'
	import { cubicOut } from 'svelte/easing'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Search, Maximize2, Minimize2 } from 'lucide-svelte'
	import { page } from '$app/state'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { getModifierKey } from '$lib/utils'
	import { Menubar } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import SettingsMenu from '$lib/components/sidebar/SettingsMenu.svelte'
	import SidebarContent from '$lib/components/sidebar/SidebarContent.svelte'
	import FavoriteMenu, { favoriteManager } from '$lib/components/sidebar/FavoriteMenu.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import SessionPicker from './SessionPicker.svelte'
	import SessionWrapper from './SessionWrapper.svelte'
	import { setSessionMode } from './sessionMode.svelte'
	import {
		createSession,
		sessionState,
		setSessionTarget,
		type SessionTarget
	} from './sessionState.svelte'

	let { children }: { children?: Snippet } = $props()

	const openSearch = getContext<(text?: string) => void>('openSearchWithPrefilledText')

	// Open animation: grow the chat pane from 0 → its resting size so the session
	// chrome slides in and the panel makes room. Svelte intro transitions don't
	// fire on content slotted inside <Pane>, so we animate the pane size directly.
	// bind:size hands control back to the splitter once the intro completes (and
	// minSize is relaxed to 0 during the intro so the from-0 frames aren't clamped).
	const CHAT_PANE_SIZE = 44
	let chatPaneSize = $state(0)
	let introDone = $state(false)
	onMount(() => {
		const start = performance.now()
		const dur = 320
		function step(now: number) {
			const t = Math.min(1, (now - start) / dur)
			chatPaneSize = CHAT_PANE_SIZE * cubicOut(t)
			if (t < 1) {
				requestAnimationFrame(step)
			} else {
				chatPaneSize = CHAT_PANE_SIZE
				introDone = true
			}
		}
		requestAnimationFrame(step)
	})

	// Full-screen panel: collapse the session chrome (rail + chat) to 0 so the
	// embedded page fills the viewport. Stash the prior size to restore on exit.
	let panelFullscreen = $state(false)
	let savedChatSize = $state(CHAT_PANE_SIZE)
	function togglePanelFullscreen() {
		if (!panelFullscreen) {
			savedChatSize = chatPaneSize
			chatPaneSize = 0
			panelFullscreen = true
		} else {
			panelFullscreen = false
			chatPaneSize = savedChatSize
		}
	}

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
	// retargeting a transient would persist it before its first message.
	$effect(() => {
		const id = activeId
		const t = parseTarget(page.url.pathname)
		if (!id || !t) return
		const s = sessionState.sessions.find((x) => x.id === id)
		if (!s || s.transient) return
		if (s.target?.kind === t.kind && s.target?.path === t.path) return
		untrack(() => setSessionTarget(id, t))
	})
</script>

<div class="h-screen w-screen flex flex-row overflow-hidden">
	<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
		<!-- Pane 1 = session chrome (sessions rail + chat). The fixed-width rail
		     plus a flex chat means dragging the (hidden) splitter resizes the chat.
		     The inner wrapper slides in from the left when session mode opens. -->
		<Pane
			bind:size={chatPaneSize}
			minSize={panelFullscreen ? 0 : introDone ? 26 : 0}
			class="flex min-h-0"
		>
			<div class="flex flex-row w-full min-h-0">
				<!-- Sessions rail, replacing the global nav sidebar. -->
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
					</div>
					<div class="flex-1 min-h-0 overflow-y-auto">
						<SessionPicker isCollapsed={false} collapsible={false} />
					</div>
					<!-- Account / instance actions (User, Settings, Workers, Logs, Help)
					     gathered under one "Settings" dropdown in the rail footer. -->
					<div class="shrink-0 border-t border-light p-2">
						<SettingsMenu />
					</div>
				</div>

				<!-- Chat: fills the rest of the pane, so the splitter resizes it. -->
				<div class="flex-1 min-w-0 flex flex-col min-h-0">
					{#if activeId}
						{#key activeId}
							<SessionWrapper
								sessionId={activeId}
								hideEditor
								onExit={() => setSessionMode(false)}
							/>
						{/key}
					{/if}
				</div>
			</div>
		</Pane>

		<!-- Pane 2 = the live Windmill page, framed like the session editor pane —
		     the nav sidebar and the page content share one rounded, bordered card. -->
		<Pane minSize={30} class="flex flex-col min-h-0">
			<!-- pl-0: the card's left edge sits flush against the splitter, so the
			     invisible handle sticks to the panel (matching the session editor pane). -->
			<div class="flex-1 min-h-0 flex flex-col {panelFullscreen ? 'p-0' : 'p-2 pl-0'}">
				<div
					class="flex flex-col flex-1 min-h-0 overflow-hidden relative bg-surface {panelFullscreen
						? ''
						: 'rounded-md border border-light'}"
				>
					<!-- Viewing breadcrumb: full-width panel header, on top of the nav sidebar. -->
					<div
						class="px-3 h-8 flex items-center gap-2 border-b border-light shrink-0 text-xs text-secondary"
					>
						<span class="text-tertiary">Viewing</span>
						<span class="font-mono truncate">{page.url.pathname}</span>
						<button
							type="button"
							onclick={togglePanelFullscreen}
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
			</div>
		</Pane>
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
