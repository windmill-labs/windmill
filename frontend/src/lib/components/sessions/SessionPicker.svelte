<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import {
		Archive,
		ArchiveRestore,
		Building,
		ChevronDown,
		ChevronRight,
		EllipsisVertical,
		Filter,
		GitFork,
		MessageSquare,
		Pencil,
		PencilLine,
		Plus,
		Trash2
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { goto } from '$lib/navigation'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { slide } from 'svelte/transition'
	import {
		createSession,
		deleteSessionsForWorkspace,
		isForkSession,
		reconcileAfterWorkspaceChange,
		renameSession,
		selectSession,
		sessionState,
		setSessionArchived,
		syncWorkspaceTo,
		type Session
	} from './sessionState.svelte'
	import { unreadCountFor } from './sessionUnread.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		getOrCreateRuntime,
		getRuntime,
		getSessionChatStatus,
		removeSession,
		resetSessionPreviewTabs
	} from './sessionRuntime.svelte'
	import SessionStatusDot from './SessionStatusDot.svelte'
	import WorkspaceIcon from '$lib/components/workspace/WorkspaceIcon.svelte'
	import { buildWorkspaceHierarchy } from '$lib/utils/workspaceHierarchy'
	import SessionFilterMenu from './SessionFilterMenu.svelte'
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton, { sidebarClasses } from '$lib/components/sidebar/MenuButton.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { currentWorkspaceRootId, workspaceRootId } from './sessionScope.svelte'
	import { page } from '$app/state'
	import { base } from '$app/paths'

	// The row icon only distinguishes "the session's fork workspace no longer
	// exists" (detached) — never the fork's ahead/behind sync state, which is
	// the fork bar's job.
	function forkDetachedFor(session: Session): boolean {
		return isUnavailableFork(session)
	}

	function isForkFor(session: Session): boolean {
		return isForkSession(session, $userWorkspaces)
	}

	// Compute the unread count for a session. Driven by the per-runtime
	// displayMessages array vs. the localStorage-backed lastSeen map;
	// both are reactive so the badge updates without polling.
	function unreadFor(session: Session): number {
		return unreadCountFor(session.id, getRuntime(session.id))
	}

	// Whether the composer for a session holds non-whitespace text. We
	// read manager.instructions directly (not the derived chat status)
	// so the draft cue still shows during streaming/needs-confirmation —
	// those override the icon slot but shouldn't hide the fact that the
	// user has unsent text in this session.
	function hasDraft(session: Session): boolean {
		const rt = getRuntime(session.id)
		return !!rt && rt.manager.instructions.trim().length > 0
	}

	// Sessions piggyback on the same dev gate as the global AI chat — when
	// the feature flag is off, the sidebar section is hidden entirely.
	const globalEnabled = isGlobalAiEnabled()

	// Only highlight the active session while the sessions page is open — elsewhere
	// `currentSessionId` lingers but no row should appear selected.
	const onSessionsPage = $derived(page.url.pathname.startsWith(`${base}/sessions`))
	const sessionActive = $derived(onSessionsPage)

	interface Props {
		isCollapsed?: boolean
		// When false, the section is always expanded (no collapse chevron) — used
		// where the picker is the whole rail rather than one sidebar section.
		collapsible?: boolean
		// Render the full workspace tree (every workspace, not just ones with
		// sessions) with clickable workspace rows on top of the nested sessions.
		workspaceTree?: boolean
		// The workspace currently being browsed (highlighted) in tree mode.
		browsedWorkspaceId?: string
		// Clicking a workspace row → browse it (preview its home, no chat).
		onBrowseWorkspace?: (workspaceId: string) => void
		// Clicking a session row → leave browse mode (bring the chat back).
		onSelectSession?: () => void
		// Drop the section's own outer padding/border so it sits flush inside a
		// parent container (e.g. gathered with Favorites/Search in the sidebar).
		embedded?: boolean
	}

	let {
		isCollapsed = false,
		collapsible = true,
		workspaceTree = false,
		browsedWorkspaceId = undefined,
		onBrowseWorkspace = undefined,
		onSelectSession = undefined,
		embedded = false
	}: Props = $props()

	const sectionCollapsed = useLocalStorageValue(
		'windmill_sessions_section_collapsed',
		false,
		'boolean'
	)
	const showArchived = useLocalStorageValue('windmill_sessions_show_archived', false, 'boolean')
	let listRoot: HTMLDivElement | undefined = $state()

	// A session's family root: the stored grouping id, else derived live.
	function sessionRootOf(s: Session): string | undefined {
		return (
			s.workspace_root_id ??
			workspaceRootId(s.workspace_id ?? s.pending_workspace_id, $userWorkspaces)
		)
	}

	// Flat list passing the archive + scope filters. Grouping for display happens
	// in `sessionGroups`; this flat view drives the runtime effect, the unread
	// total, and keyboard navigation.
	const visibleSessions = $derived(
		sessionState.sessions.filter((s) => {
			if (s.transient) return false
			// The open session always stays in the list, ignoring both filters.
			if (s.id === sessionState.currentSessionId) return true
			if (s.archived && !showArchived.val) return false
			// Scope to the current workspace family only.
			const currentRoot = $currentWorkspaceRootId
			if (currentRoot && sessionRootOf(s) !== currentRoot) return false
			return true
		})
	)

	// Sessions grouped by workspace family for display, each group newest-first.
	// Family order is stable (by most-recent activity) and deliberately NOT tied
	// to the current workspace: pinning the active family first reshuffled the
	// whole list on every workspace switch, which is disorienting.
	const sessionGroups = $derived.by(() => {
		const byRoot = new Map<string, Session[]>()
		for (const s of visibleSessions) {
			const root = sessionRootOf(s) ?? s.workspace_id ?? s.pending_workspace_id ?? ''
			const arr = byRoot.get(root)
			if (arr) arr.push(s)
			else byRoot.set(root, [s])
		}
		const groups = [...byRoot.entries()].map(([rootId, sessions]) => {
			sessions.sort((a, b) => b.createdAt - a.createdAt)
			return {
				rootId,
				name: $userWorkspaces.find((w) => w.id === rootId)?.name || rootId || 'Workspace',
				sessions,
				mostRecent: sessions[0]?.createdAt ?? 0
			}
		})
		groups.sort((a, b) => b.mostRecent - a.mostRecent)
		return groups
	})

	// Workspace tree scoped to the current family (root + its forks), so the rail
	// only shows the workspaces in the family we're in — not every workspace.
	const workspaceTreeItems = $derived.by(() => {
		const all = buildWorkspaceHierarchy($userWorkspaces)
		const currentRoot = $currentWorkspaceRootId
		if (!currentRoot) return all
		return all.filter((i) => workspaceRootId(i.workspace.id, $userWorkspaces) === currentRoot)
	})

	// Sessions keyed by their exact workspace, newest-first — nested under each
	// workspace node in tree mode.
	const sessionsByWorkspace = $derived.by(() => {
		const map = new Map<string, Session[]>()
		for (const s of visibleSessions) {
			const wsId = s.workspace_id ?? s.pending_workspace_id
			if (!wsId) continue
			const arr = map.get(wsId)
			if (arr) arr.push(s)
			else map.set(wsId, [s])
		}
		for (const arr of map.values()) arr.sort((a, b) => b.createdAt - a.createdAt)
		return map
	})

	// Collapsed workspaces in the tree, persisted to localStorage so the state is
	// shared between the rail and the collapsed popover (separate picker instances)
	// and survives reloads. A collapsed workspace hides its sessions + fork subtree.
	const collapsedWorkspaces = useLocalStorageValue<string[]>(
		'windmill_sessions_collapsed_workspaces',
		[]
	)
	function isWorkspaceCollapsed(id: string): boolean {
		return collapsedWorkspaces.val.includes(id)
	}
	function toggleWorkspaceCollapsed(id: string) {
		collapsedWorkspaces.val = isWorkspaceCollapsed(id)
			? collapsedWorkspaces.val.filter((x) => x !== id)
			: [...collapsedWorkspaces.val, id]
	}

	// Ids of tree items hidden because an ancestor workspace is collapsed. Computed
	// in pre-order: once a collapsed node is seen, everything deeper is hidden until
	// the depth returns to its level or shallower.
	const hiddenWorkspaceIds = $derived.by(() => {
		const hidden = new Set<string>()
		let collapseDepth = Infinity
		for (const item of workspaceTreeItems) {
			if (item.depth > collapseDepth) {
				hidden.add(item.workspace.id)
				continue
			}
			collapseDepth = isWorkspaceCollapsed(item.workspace.id) ? item.depth : Infinity
		}
		return hidden
	})

	// Family labels are redundant when scoped to a single family — only show them
	// if the active-session override surfaces a second family.
	const showGroupHeaders = $derived(sessionGroups.length > 1)

	const archivedCount = $derived(
		sessionState.sessions.filter((s) => {
			if (!s.archived || s.transient) return false
			const currentRoot = $currentWorkspaceRootId
			return (
				!currentRoot || sessionRootOf(s) === currentRoot || s.id === sessionState.currentSessionId
			)
		}).length
	)

	// Sum of unread across every visible session — surfaced on the
	// collapsed-sidebar chat icon so the user sees there's pending
	// AI activity in some session without expanding the sidebar.
	const totalUnread = $derived(visibleSessions.reduce((acc, s) => acc + unreadFor(s), 0))

	// Clear any persisted collapsed state while the list is empty. The
	// empty-state header is a plain label with no toggle, so a collapse
	// carried over from a previous session (or another workspace) would
	// otherwise hide the user's first new session with no way to expand
	// it. Resetting here keeps the section expanded by default whenever
	// the first session arrives. Guarded on the current value so it writes
	// once (true → false) rather than looping.
	$effect(() => {
		if (visibleSessions.length === 0 && sectionCollapsed.val) {
			sectionCollapsed.val = false
		}
	})

	// Eagerly create a runtime per VISIBLE session so the status dot reflects
	// the persisted chat (last message, pending confirmation, etc.) without
	// requiring the user to open the session first. Sessions outside the
	// current workspace scope are left cold to avoid opening IDB connections
	// for unrelated work.
	$effect(() => {
		for (const session of visibleSessions) {
			getOrCreateRuntime(session)
		}
	})

	function isUnavailableFork(session: Session): boolean {
		return !!session.workspace_id && !$userWorkspaces.find((w) => w.id === session.workspace_id)
	}

	async function activate(session: Session, restoreFocus: boolean = false) {
		// Picking a session leaves browse mode so the chat comes back.
		onSelectSession?.()
		selectSession(session.id)
		// The global workspaceStore is intentionally NOT switched here: a session
		// runs against its own workspace via the chat manager's workspace resolver,
		// so opening one must not change the user's active (navigation) workspace.
		// Open the dedicated sessions page; its preview panel iframes the
		// session's view (captured page / editor target).
		await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
		if (restoreFocus) {
			// goto() resets focus to <body> — put it back on the active session button
			// so subsequent arrow keys keep navigating the list.
			requestAnimationFrame(() => {
				const selected = listRoot?.querySelector<HTMLButtonElement>(
					'button[data-session-button][aria-selected="true"]'
				)
				selected?.focus()
			})
		}
	}

	async function createAndOpen() {
		const fresh = createSession()
		// A new session opened from a Windmill page adopts that page as its first
		// preview tab (resetSessionPreviewTabs handles a reused transient whose
		// tabs still show a previous destination). Skip when already on the
		// sessions page (nothing meaningful to capture) so the preview seeds from
		// the session's editor target (or stays empty) instead.
		if (!onSessionsPage) {
			const url = page.url.pathname + page.url.search
			resetSessionPreviewTabs(fresh.id, url)
		}
		await activate(fresh)
	}

	let editingId: string | undefined = $state(undefined)
	let renameDraft = $state('')

	function startRename(session: Session) {
		editingId = session.id
		renameDraft = session.summary ?? ''
	}

	function commitRename() {
		const id = editingId
		if (!id) return
		renameSession(id, renameDraft)
		editingId = undefined
	}

	function cancelRename() {
		editingId = undefined
	}

	let pendingDelete: Session | undefined = $state(undefined)
	// Default off: deleting a fork workspace is destructive and not what deleting a
	// session implies. The user can tick it in the modal to also drop the fork.
	let deleteAlsoFork = $state(false)
	// Fork workspace tied to `pendingDelete`, if any, and still accessible.
	const pendingDeleteForkId = $derived.by(() => {
		const wsId = pendingDelete?.workspace_id
		if (!wsId) return undefined
		const ws = $userWorkspaces.find((w) => w.id === wsId)
		// Fork = prefix OR parent (so an orphaned wm-fork- fork still qualifies); exclude persistent
		// dev workspaces, which are not ephemeral session forks.
		if (!ws || !workspaceIsFork(wsId, $userWorkspaces)) return undefined
		if (ws.is_dev_workspace) return undefined
		return wsId
	})

	async function handleConfirmedDelete() {
		const session = pendingDelete
		const forkToDelete = deleteAlsoFork ? pendingDeleteForkId : undefined
		// Capture the fork's parent before the workspace list is refreshed
		// below — afterwards the fork is gone from $userWorkspaces and the
		// lookup would return undefined.
		const forkParentId = forkToDelete
			? $userWorkspaces.find((w) => w.id === forkToDelete)?.parent_workspace_id
			: undefined
		pendingDelete = undefined
		deleteAlsoFork = false
		if (!session) return
		const wasActive = sessionState.currentSessionId === session.id
		removeSession(session.id)
		if (forkToDelete) {
			try {
				await WorkspaceService.deleteWorkspace({ workspace: forkToDelete })
				await deleteSessionsForWorkspace(forkToDelete)
				sendUserToast(`Deleted forked workspace ${forkToDelete}`)
				await reconcileAfterWorkspaceChange()
			} catch (e: any) {
				sendUserToast(`Failed to delete fork ${forkToDelete}: ${e?.body ?? e}`, true)
			}
		}
		// If the deleted fork was the active workspace, fall back to its parent
		// so the user isn't stranded on a workspace that no longer exists.
		if (forkToDelete && forkParentId && $workspaceStore === forkToDelete) {
			syncWorkspaceTo(forkParentId)
		}
		if (wasActive) {
			const next = sessionState.sessions[0]
			if (next) await activate(next)
			else {
				// No sessions left — create a fresh one and navigate to it. The page
				// derives the visible session from the `session_name` query, so just
				// clearing currentSessionId would strand the URL on the deleted session
				// and render its not-found state instead of a ready-to-type composer.
				const fresh = createSession()
				await goto(`/sessions?session_name=${encodeURIComponent(fresh.name)}`)
			}
		}
	}

	function focusAt(index: number) {
		const buttons = listRoot
			? Array.from(listRoot.querySelectorAll<HTMLButtonElement>('button[data-session-button]'))
			: []
		if (buttons.length === 0) return
		const wrapped = ((index % buttons.length) + buttons.length) % buttons.length
		buttons[wrapped]?.focus()
	}

	function handleListKeydown(e: KeyboardEvent) {
		if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') {
			return
		}
		const buttons = listRoot
			? Array.from(listRoot.querySelectorAll<HTMLButtonElement>('button[data-session-button]'))
			: []
		if (buttons.length === 0) return
		const current = buttons.indexOf(document.activeElement as HTMLButtonElement)
		e.preventDefault()
		if (e.key === 'ArrowDown') focusAt(current < 0 ? 0 : current + 1)
		else if (e.key === 'ArrowUp') focusAt(current < 0 ? buttons.length - 1 : current - 1)
		else if (e.key === 'Home') focusAt(0)
		else if (e.key === 'End') focusAt(buttons.length - 1)
	}

	const menuItemBase = twMerge(
		'text-secondary text-left font-normal text-xs',
		'flex flex-row items-center gap-2 px-3 py-1.5 w-full',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
</script>

{#if !globalEnabled}
	<!-- Sessions hidden until the global-ai dev gate is enabled. When AI is
	     unavailable (no provider configured or disabled in the user's settings)
	     the section still shows — the per-session chat input is disabled with an
	     explanatory message, mirroring the sidebar AI chat. -->
{:else if isCollapsed}
	<div class={embedded ? '' : 'px-2 pt-3 pb-2 border-b border-light'}>
		<Menubar>
			{#snippet children({ createMenu })}
				<Menu {createMenu} usePointerDownOutside submenuSafe>
					{#snippet triggr({ trigger })}
						<div class="relative">
							<MenuButton
								class="!text-xs"
								icon={MessageSquare}
								label="AI sessions"
								{isCollapsed}
								{trigger}
							/>
							{#if totalUnread > 0}
								<span
									class="absolute top-1 right-1 pointer-events-none inline-block w-2 h-2 rounded-full bg-surface-accent-primary"
									aria-label="{totalUnread} unread message{totalUnread === 1
										? ''
										: 's'} across all sessions"
								></span>
							{/if}
						</div>
					{/snippet}
					{#snippet children({ item, builders })}
						<div class="divide-y min-w-48" role="none">
							<div class="py-1" role="none">
								<MenuItem class={menuItemBase} onClick={createAndOpen} {item}>
									<Plus size={14} />
									New session
								</MenuItem>
							</div>
							<div class="py-1" role="none">
								<SessionFilterMenu
									{builders}
									bind:showArchived={showArchived.val}
									{archivedCount}
								/>
							</div>
							<div class="py-1" role="none">
								{#each sessionGroups as group (group.rootId)}
									{#if showGroupHeaders}
										<div
											class="px-3 pt-1.5 pb-0.5 text-[0.5rem] uppercase text-tertiary truncate"
											role="none"
											title={group.name}
										>
											{group.name}
										</div>
									{/if}
									{#each group.sessions as session (session.id)}
										{@const runtime = getRuntime(session.id)}
										{@const status = runtime ? getSessionChatStatus(runtime) : 'idle'}
										{@const isSelected =
											sessionActive && session.id === sessionState.currentSessionId}
										{@const unread = unreadFor(session)}
										{@const draft = hasDraft(session)}
										<MenuItem
											class={twMerge(
												menuItemBase,
												isSelected
													? twMerge(sidebarClasses.selectedBg, sidebarClasses.selectedText)
													: ''
											)}
											onClick={() => activate(session)}
											{item}
										>
											<SessionStatusDot
												{status}
												isFork={isForkFor(session)}
												forkDetached={forkDetachedFor(session)}
											/>
											<span
												class={twMerge(
													'truncate flex-1 text-left',
													unread > 0 ? 'font-semibold text-primary' : ''
												)}
											>
												{session.summary ?? 'Untitled session'}
											</span>
											{#if draft || unread > 0}
												<span class="ml-auto shrink-0 inline-flex items-center gap-1">
													{#if draft}
														<PencilLine class="w-3 h-3 text-tertiary" aria-label="Unsent draft" />
													{/if}
													{#if unread > 0}
														<span
															class="inline-flex items-center justify-center rounded-full bg-surface-accent-primary text-white font-medium leading-none min-w-4 h-4 px-1 text-[10px]"
															aria-label="{unread} unread message{unread === 1 ? '' : 's'}"
														>
															{unread > 9 ? '9+' : unread}
														</span>
													{/if}
												</span>
											{/if}
										</MenuItem>
									{/each}
								{/each}
							</div>
						</div>
					{/snippet}
				</Menu>
			{/snippet}
		</Menubar>
	</div>
{:else}
	<div
		class="flex flex-col gap-1 {embedded ? '' : 'px-2 pt-3 pb-2'} {!embedded && collapsible
			? 'border-b border-light'
			: ''}"
	>
		<div class="flex flex-row items-center justify-between pl-1 pr-0.5">
			{#if collapsible && visibleSessions.length > 0}
				<button
					type="button"
					onclick={() => (sectionCollapsed.val = !sectionCollapsed.val)}
					class="text-secondary text-[0.5rem] uppercase flex flex-row items-center gap-1 rounded px-1 -mx-1 py-0.5 hover:bg-surface-hover focus:outline-none"
					aria-expanded={!sectionCollapsed.val}
				>
					AI sessions
					{#if sectionCollapsed.val}
						<ChevronRight size={10} />
					{:else}
						<ChevronDown size={10} />
					{/if}
				</button>
			{:else}
				<!-- No sessions yet: render the label as plain text (no collapse toggle). -->
				<span
					class="text-secondary text-[0.5rem] uppercase flex flex-row items-center gap-1 px-1 -mx-1 py-0.5"
				>
					AI sessions
				</span>
			{/if}
			<div class="flex flex-row items-center gap-0.5">
				<button
					type="button"
					title="New session"
					aria-label="New session"
					onclick={createAndOpen}
					class="inline-flex items-center justify-center w-5 h-5 rounded text-tertiary hover:bg-surface-hover hover:text-primary"
				>
					<Plus size={12} />
				</button>
				<Popover placement="bottom-end" usePointerDownOutside disableFocusTrap class="inline-flex">
					{#snippet trigger()}
						<button
							type="button"
							title="Filter sessions"
							aria-label="Filter sessions"
							class="inline-flex items-center justify-center w-5 h-5 rounded text-tertiary hover:bg-surface-hover hover:text-primary {showArchived.val
								? 'text-emphasis'
								: ''}"
						>
							<Filter size={12} />
						</button>
					{/snippet}
					{#snippet content()}
						<div
							class="w-56 p-2 bg-surface-tertiary dark:border rounded-md shadow-lg flex flex-col gap-2"
						>
							<div class="flex flex-col gap-0.5">
								<Toggle
									bind:checked={showArchived.val}
									size="xs"
									options={{ right: 'Show archived' }}
								/>
								{#if archivedCount > 0}
									<span class="text-2xs text-tertiary pl-1">
										{archivedCount} archived session{archivedCount === 1 ? '' : 's'}
									</span>
								{/if}
							</div>
						</div>
					{/snippet}
				</Popover>
			</div>
		</div>
		{#if !collapsible || !sectionCollapsed.val}
			<div
				bind:this={listRoot}
				transition:slide={{ duration: 180 }}
				class={twMerge(
					'flex flex-col gap-0.5 overflow-y-auto',
					// In the rail the picker is the whole list and the rail scrolls;
					// only cap height when it's one section among others (normal sidebar).
					collapsible ? 'max-h-[40vh]' : ''
				)}
				onkeydown={handleListKeydown}
				role="listbox"
				tabindex="-1"
			>
				{#snippet vline()}
					<span class="relative w-3.5 shrink-0 self-stretch">
						<span class="absolute inset-y-0 left-1/2 w-px bg-surface-tertiary"></span>
					</span>
				{/snippet}
				{#snippet sessionRow(session, indented, treeDepth)}
					{@const runtime = getRuntime(session.id)}
					{@const status = runtime ? getSessionChatStatus(runtime) : 'idle'}
					{@const isSelected = sessionActive && session.id === sessionState.currentSessionId}
					{@const isEditing = editingId === session.id}
					{@const unread = unreadFor(session)}
					{@const draft = hasDraft(session)}
					<div
						class={twMerge(
							'flex flex-row group rounded',
							treeDepth === undefined ? 'items-center' : 'items-stretch',
							isSelected ? sidebarClasses.selectedBg : 'hover:bg-surface-hover',
							session.archived ? 'opacity-60' : '',
							// Family mode indents under the header; tree mode uses guide columns.
							treeDepth === undefined && indented ? 'pl-5' : ''
						)}
					>
						{#if treeDepth !== undefined}
							{#each Array(treeDepth) as _}{@render vline()}{/each}
						{/if}
						{#if isEditing}
							<span class="flex flex-row items-center gap-2 flex-1 px-2 py-1 min-w-0">
								{#if treeDepth === undefined}
									<SessionStatusDot
										{status}
										isFork={isForkFor(session)}
										forkDetached={forkDetachedFor(session)}
									/>
								{/if}
								<TextInput
									bind:value={renameDraft}
									size="xs"
									unifiedHeight={false}
									class="flex-1 min-w-0 !bg-transparent !border-0 !border-transparent !shadow-none focus:!ring-0 px-0 text-xs font-normal text-primary"
									inputProps={{
										type: 'text',
										placeholder: 'Untitled session',
										autofocus: true,
										spellcheck: false,
										onkeydown: (e) => {
											if (e.key === 'Enter') commitRename()
											else if (e.key === 'Escape') cancelRename()
										},
										onblur: commitRename
									}}
								/>
							</span>
						{:else}
							<button
								type="button"
								data-session-button
								role="option"
								aria-selected={isSelected}
								onclick={() => activate(session)}
								class={twMerge(
									'flex flex-row items-center gap-2 text-left text-xs font-normal focus:outline-none flex-1 min-w-0 px-2 py-1',
									isSelected ? sidebarClasses.selectedText : 'text-secondary'
								)}
							>
								{#if treeDepth === undefined}
									<SessionStatusDot
										{status}
										isFork={isForkFor(session)}
										forkDetached={forkDetachedFor(session)}
									/>
								{/if}
								<span class="truncate flex-1">{session.summary ?? 'Untitled session'}</span>
								{#if draft || unread > 0}
									<span class="shrink-0 inline-flex items-center gap-1">
										{#if draft}
											<PencilLine class="w-3 h-3 text-tertiary" aria-label="Unsent draft" />
										{/if}
										{#if unread > 0}
											<span
												class="inline-flex items-center justify-center rounded-full bg-surface-accent-primary text-white font-medium leading-none min-w-4 h-4 px-1 text-[10px]"
												aria-label="{unread} unread message{unread === 1 ? '' : 's'}"
											>
												{unread > 9 ? '9+' : unread}
											</span>
										{/if}
									</span>
								{/if}
							</button>
							<div
								class="opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity pr-0.5"
							>
								<DropdownV2
									fixedHeight={false}
									placement="bottom-end"
									enableFlyTransition
									items={[
										{
											displayName: 'Rename',
											icon: Pencil,
											action: () => startRename(session)
										},
										...(session.archived
											? // No Unarchive when the workspace is gone — it can't persist
												// (putSession guard) and reconcile would re-archive it.
												isUnavailableFork(session)
												? []
												: [
														{
															displayName: 'Unarchive',
															icon: ArchiveRestore,
															action: () => setSessionArchived(session.id, false)
														}
													]
											: [
													{
														displayName: 'Archive',
														icon: Archive,
														action: () => setSessionArchived(session.id, true)
													}
												]),
										{
											displayName: 'Delete',
											icon: Trash2,
											type: 'delete',
											action: () => (pendingDelete = session)
										}
									]}
								>
									{#snippet buttonReplacement()}
										<span
											class="inline-flex items-center justify-center w-5 h-5 rounded text-tertiary hover:bg-surface-hover hover:text-primary"
											title="More"
										>
											<EllipsisVertical size={14} />
										</span>
									{/snippet}
								</DropdownV2>
							</div>
						{/if}
					</div>
				{/snippet}
				{#if workspaceTree}
					{#each workspaceTreeItems as item, wi (item.workspace.id)}
						{#if !hiddenWorkspaceIds.has(item.workspace.id)}
							{@const wsSessions = sessionsByWorkspace.get(item.workspace.id) ?? []}
							{@const collapsed = isWorkspaceCollapsed(item.workspace.id)}
							{@const collapsible = item.hasChildren || wsSessions.length > 0}
							<!-- Workspace = folder. Stroke-colored building/fork glyph; the guide
							     columns render the workspace tree (fork nesting). A chevron
							     collapses the workspace (hides its sessions + fork subtree). -->
							<div
								class={twMerge(
									'flex items-stretch w-full rounded',
									wi > 0 && item.depth === 0 ? 'mt-3' : '',
									browsedWorkspaceId === item.workspace.id
										? sidebarClasses.selectedBg
										: 'hover:bg-surface-hover'
								)}
							>
								{#each Array(item.depth) as _}{@render vline()}{/each}
								<button
									type="button"
									onclick={() => onBrowseWorkspace?.(item.workspace.id)}
									title={item.workspace.name}
									class="flex items-center gap-1.5 py-1 pl-1 pr-2 min-w-0 flex-1 text-left"
								>
									{#if item.isForked}
										<GitFork size={14} class="shrink-0" style="color: {item.workspace.color}" />
									{:else}
										<Building size={14} class="shrink-0" style="color: {item.workspace.color}" />
									{/if}
									<span
										class={twMerge(
											'text-xs truncate font-normal',
											browsedWorkspaceId === item.workspace.id
												? 'text-emphasis font-medium'
												: 'text-primary'
										)}>{item.workspace.name}</span
									>
								</button>
								{#if collapsible}
									<button
										type="button"
										onclick={() => toggleWorkspaceCollapsed(item.workspace.id)}
										title={collapsed ? 'Expand' : 'Collapse'}
										aria-label={collapsed ? 'Expand workspace' : 'Collapse workspace'}
										class="shrink-0 flex items-center justify-center w-6 text-tertiary hover:text-primary"
									>
										{#if collapsed}
											<ChevronRight size={12} />
										{:else}
											<ChevronDown size={12} />
										{/if}
									</button>
								{/if}
							</div>
							{#if !collapsed}
								{#each wsSessions as session (session.id)}
									{@render sessionRow(session, true, item.depth + 1)}
								{/each}
							{/if}
						{/if}
					{/each}
				{:else}
					{#each sessionGroups as group, groupIdx (group.rootId)}
						{#if showGroupHeaders}
							{@const groupWs = $userWorkspaces.find((w) => w.id === group.rootId)}
							<div
								class={twMerge(
									'flex items-center gap-2 px-2 pt-2 pb-1 min-w-0',
									// Space families apart so group boundaries read clearly.
									groupIdx > 0 ? 'mt-4' : ''
								)}
								title={group.name}
							>
								<WorkspaceIcon workspaceColor={groupWs?.color} size={12} />
								<span class="text-xs font-medium text-secondary truncate">{group.name}</span>
							</div>
						{/if}
						{#each group.sessions as session (session.id)}
							{@render sessionRow(session, showGroupHeaders, undefined)}
						{/each}
					{/each}
				{/if}
			</div>
		{/if}
	</div>
{/if}

<ConfirmationModal
	open={!!pendingDelete}
	title="Delete session"
	confirmationText="Delete"
	onConfirmed={handleConfirmedDelete}
	onCanceled={() => {
		pendingDelete = undefined
		deleteAlsoFork = false
	}}
>
	<div class="flex flex-col gap-3">
		<p>
			Delete session <span class="font-medium text-primary"
				>{pendingDelete?.summary ?? pendingDelete?.name}</span
			>? This cannot be undone.
		</p>
		{#if pendingDeleteForkId}
			<div class="flex items-start gap-2 border rounded-md p-3 bg-surface-secondary">
				<Toggle size="xs" bind:checked={deleteAlsoFork} />
				<div class="flex flex-col">
					<span class="text-xs font-medium text-primary"
						>Also delete forked workspace <span class="font-mono">{pendingDeleteForkId}</span></span
					>
					<span class="text-3xs text-tertiary"
						>The fork won't be reachable from any other session — leaving it would orphan it.</span
					>
				</div>
			</div>
		{/if}
	</div>
</ConfirmationModal>
