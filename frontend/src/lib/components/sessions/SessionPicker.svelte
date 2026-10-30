<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import { Button } from '$lib/components/common'
	import SidebarSectionHeader from '$lib/components/sidebar/SidebarSectionHeader.svelte'
	import {
		Archive,
		ArchiveRestore,
		Building,
		ChevronDown,
		ChevronRight,
		Clock,
		EllipsisVertical,
		GitFork,
		ListChecks,
		MessageSquare,
		Pencil,
		PencilLine,
		Plus,
		Rows3,
		Trash2
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { goto } from '$lib/navigation'
	import { leaveForHome, openDraftOnHome, openNewSession } from './sessionSwitch.svelte'
	import { homeComposerContext } from './homeComposerContext.svelte'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { slide } from 'svelte/transition'
	import {
		deleteSessionsForWorkspace,
		homeDraft,
		isForkSession,
		opensOnHome,
		reconcileAfterWorkspaceChange,
		renameSession,
		selectSession,
		sessionPageHref,
		sessionLastActivityAt,
		sessionState,
		setSessionArchived,
		syncWorkspaceTo,
		withOpenSessionTeardown,
		type Session
	} from './sessionState.svelte'
	import { unreadCountFor } from './sessionUnread.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		ensureSessionChatPeek,
		getRuntime,
		getSessionChatPeek,
		getSessionChatStatus,
		removeSession
	} from './sessionRuntime.svelte'
	import SessionStatusDot from './SessionStatusDot.svelte'
	import UnreadCountBadge from '$lib/components/common/badge/UnreadCountBadge.svelte'
	import { buildWorkspaceHierarchy } from '$lib/utils/workspaceHierarchy'
	import SessionFilterMenu from './SessionFilterMenu.svelte'
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton, { sidebarClasses } from '$lib/components/sidebar/MenuButton.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { currentWorkspaceRootId, workspaceRootId } from './sessionScope.svelte'
	import { page } from '$app/state'
	import DevWorkspaceBadge from '$lib/components/DevWorkspaceBadge.svelte'
	import {
		dateBucket,
		GROUP_BY_OPTIONS,
		LAST_ACTIVITY_OPTIONS,
		type GroupBy
	} from './sessionFilters'

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
		return unreadCountFor(session.id, getRuntime(session.id), getSessionChatPeek(session.id))
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

	// Sessions share the beta opt-out gate with the global AI chat — when the
	// user opted out, the sidebar section is hidden entirely.
	const globalEnabled = isGlobalAiEnabled()

	// The activity cutoff and the date buckets are wall-clock reads, which Svelte
	// cannot track: without this the list would keep a session past its cutoff, and
	// keep yesterday's header on today's rows, until some unrelated write forced a
	// re-derive. Ticking a tracked timestamp bounds that staleness to a minute.
	let clock = $state(Date.now())
	$effect(() => {
		const handle = setInterval(() => (clock = Date.now()), 60_000)
		return () => clearInterval(handle)
	})

	// The row the page is showing: the open session on its page, the draft in Home's
	// composer on Home. Elsewhere `currentSessionId` lingers but no row is selected.
	const activeRowId = $derived(
		page.url.pathname.startsWith('/sessions')
			? sessionState.currentSessionId
			: page.url.pathname === '/'
				? homeDraft.id
				: undefined
	)

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
	// Hide sessions untouched for longer than this many days. 0 = no cutoff.
	const lastActivityDays = useLocalStorageValue('windmill_sessions_last_activity_days', 0, 'number')
	// How the list is carved into groups. 'none' keeps the family grouping (headers
	// only when a second family shows up); the others always draw headers.
	const groupBy = useLocalStorageValue<GroupBy>('windmill_sessions_group_by', 'none', 'string')
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
			// A blank nobody has typed in is Home's composer at rest, not a session yet.
			if (s.transient) return false
			// Pending (unsent) sessions show like any other, so several drafts can be
			// set up in parallel; they group by pending_workspace_id via sessionRootOf.
			// The row on screen always stays in the list, ignoring every filter.
			if (s.id === activeRowId) return true
			if (s.archived && !showArchived.val) return false
			if (lastActivityDays.val > 0) {
				const cutoff = clock - lastActivityDays.val * 24 * 60 * 60 * 1000
				if (sessionLastActivityAt(s) < cutoff) return false
			}
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
		const all = buildWorkspaceHierarchy($userWorkspaces, $userStore?.email)
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

	// The list as the user chose to see it. `workspaceId` is set on the groups that
	// stand for a workspace — the ones whose header offers "new session here".
	type DisplayGroup = {
		key: string
		label: string
		workspaceId?: string
		sessions: Session[]
		showHeader: boolean
	}

	const displayGroups: DisplayGroup[] = $derived.by(() => {
		if (groupBy.val === 'none') {
			return sessionGroups.map((g) => ({
				key: g.rootId,
				label: g.name,
				workspaceId: g.rootId || undefined,
				sessions: g.sessions,
				showHeader: showGroupHeaders
			}))
		}
		const byActivity = (a: Session, b: Session) =>
			sessionLastActivityAt(b) - sessionLastActivityAt(a)
		if (groupBy.val === 'date') {
			const now = clock
			const buckets = new Map<string, { label: string; rank: number; sessions: Session[] }>()
			for (const s of visibleSessions) {
				const b = dateBucket(sessionLastActivityAt(s), now)
				const entry = buckets.get(b.key)
				if (entry) entry.sessions.push(s)
				else buckets.set(b.key, { label: b.label, rank: b.rank, sessions: [s] })
			}
			return [...buckets.entries()]
				.sort((a, b) => a[1].rank - b[1].rank)
				.map(([key, v]) => ({
					key,
					label: v.label,
					sessions: v.sessions.sort(byActivity),
					showHeader: true
				}))
		}
		// Fork: one group per actual workspace — the root and each fork stand alone,
		// unlike the family grouping that folds a whole fork tree into its root.
		const byWorkspace = new Map<string, Session[]>()
		for (const s of visibleSessions) {
			const wsId = s.workspace_id ?? s.pending_workspace_id ?? ''
			const arr = byWorkspace.get(wsId)
			if (arr) arr.push(s)
			else byWorkspace.set(wsId, [s])
		}
		// A lone workspace names itself — its header would label the whole list.
		const headed = byWorkspace.size > 1
		return [...byWorkspace.entries()]
			.map(([wsId, sessions]) => {
				sessions.sort(byActivity)
				// Sessions on a deleted fork keep its id; only a workspace the user
				// still has can host a new session (putSession drops the rest), so
				// `workspaceId` — which drives the header `+` — stays unset for those.
				const known = $userWorkspaces.find((w) => w.id === wsId)
				return {
					key: wsId,
					label: known?.name || wsId || 'No workspace yet',
					workspaceId: known ? wsId : undefined,
					sessions,
					showHeader: headed
				}
			})
			.sort((a, b) => sessionLastActivityAt(b.sessions[0]) - sessionLastActivityAt(a.sessions[0]))
	})

	// What "Show archived" would actually reveal: every filter the list applies
	// except the archive one, so the count never promises rows the activity
	// cutoff would then hide.
	const archivedCount = $derived(
		sessionState.sessions.filter((s) => {
			if (!s.archived || s.transient) return false
			if (s.id === activeRowId) return true
			if (lastActivityDays.val > 0) {
				const cutoff = clock - lastActivityDays.val * 24 * 60 * 60 * 1000
				if (sessionLastActivityAt(s) < cutoff) return false
			}
			const currentRoot = $currentWorkspaceRootId
			return !currentRoot || sessionRootOf(s) === currentRoot
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

	// Read each VISIBLE session's stored chat so the status dot and unread count
	// reflect it without the user opening the session. A runtime per listed
	// session would hold every listed chat in memory for as long as the page lives.
	$effect(() => {
		for (const session of visibleSessions) {
			if (!getRuntime(session.id)) void ensureSessionChatPeek(session)
		}
	})

	function isUnavailableFork(session: Session): boolean {
		return !!session.workspace_id && !$userWorkspaces.find((w) => w.id === session.workspace_id)
	}

	async function activate(session: Session, restoreFocus: boolean = false) {
		// Picking a session leaves browse mode so the chat comes back.
		onSelectSession?.()
		if (opensOnHome(session)) {
			await openDraftOnHome(session.id)
			return
		}
		selectSession(session.id)
		// The global workspaceStore is intentionally NOT switched here: a session
		// runs against its own workspace via the chat manager's workspace resolver,
		// so opening one must not change the user's active (navigation) workspace.
		// Open the dedicated sessions page; its preview panel iframes the
		// session's view (captured page / editor target).
		await goto(sessionPageHref(session.id))
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

	// The rail menu hands focus back to its trigger once it has closed, after New session has
	// focused Home's composer: the composer takes it back instead.
	let newSessionFromRail = false
	function railMenuCloseFocus(trigger?: HTMLElement | null): HTMLElement | null {
		if (!newSessionFromRail) return trigger ?? null
		newSessionFromRail = false
		homeComposerContext.focus()
		return null
	}

	// The `+` on a workspace group header: a new session in that group's workspace,
	// which the header switches to, since Home's draft acts on the header's workspace.
	async function createAndOpenIn(workspaceId: string) {
		await openNewSession(workspaceId)
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
	// Fork workspace tied to a session, if any, and still accessible.
	function forkWorkspaceIdFor(session: Session | undefined): string | undefined {
		const wsId = session?.workspace_id
		if (!wsId) return undefined
		const ws = $userWorkspaces.find((w) => w.id === wsId)
		// Fork = prefix OR parent (so an orphaned wm-fork- fork still qualifies); exclude persistent
		// dev workspaces, which are not ephemeral session forks.
		if (!ws || !workspaceIsFork(wsId, $userWorkspaces)) return undefined
		if (ws.is_dev_workspace) return undefined
		return wsId
	}
	const pendingDeleteForkId = $derived(forkWorkspaceIdFor(pendingDelete))

	// Batch selection: "Edit sessions" in the header menu turns the rows into
	// checkboxes so several sessions can be deleted in one pass.
	let selectionMode = $state(false)
	let selectedIds = $state<string[]>([])
	let batchDeleteOpen = $state(false)

	const selectableIds = $derived(visibleSessions.map((s) => s.id))
	const allSelected = $derived(
		selectableIds.length > 0 && selectedIds.length === selectableIds.length
	)
	const selectedForkCount = $derived(
		selectedIds.filter((id) => forkWorkspaceIdFor(sessionState.sessions.find((s) => s.id === id)))
			.length
	)
	// A mixed selection archives; only an all-archived selection reads as "undo
	// that", so that's the single case the button flips to Unarchive.
	const allSelectedArchived = $derived(
		selectedIds.length > 0 &&
			selectedIds.every((id) => sessionState.sessions.find((s) => s.id === id)?.archived)
	)

	// Drop selections that left the list (archive filter flipped, workspace family
	// switched) so the count never claims more than the visible checkboxes.
	$effect(() => {
		if (!selectionMode) return
		const visible = new Set(selectableIds)
		const pruned = selectedIds.filter((id) => visible.has(id))
		if (pruned.length !== selectedIds.length) selectedIds = pruned
	})

	function enterSelectionMode() {
		selectionMode = true
		selectedIds = []
		// Checkboxes are unreachable while the section is folded away.
		sectionCollapsed.val = false
	}

	function exitSelectionMode() {
		selectionMode = false
		selectedIds = []
		rangeAnchorId = undefined
	}

	// A checkbox's `click` carries the modifier keys and its `change` carries the
	// state, and only `change` may drive the value — preventing the click's default
	// leaves the input's own checked flag out of step with the prop. So the click
	// only records whether shift was down, for the change handler that follows it.
	let shiftHeldOnClick = false

	// Anchor for shift-click: the row whose checkbox was last set, as in a file list.
	let rangeAnchorId: string | undefined = $state(undefined)

	// Every visible row in display order, which is what a shift-range spans — it
	// runs across group boundaries, matching what the user sees.
	const orderedIds = $derived(displayGroups.flatMap((g) => g.sessions.map((s) => s.id)))

	function toggleSelected(id: string, extendRange = false) {
		const anchor = rangeAnchorId
		rangeAnchorId = id
		if (extendRange && anchor && anchor !== id) {
			const from = orderedIds.indexOf(anchor)
			const to = orderedIds.indexOf(id)
			if (from !== -1 && to !== -1) {
				const range = orderedIds.slice(Math.min(from, to), Math.max(from, to) + 1)
				const merged = new Set([...selectedIds, ...range])
				selectedIds = orderedIds.filter((x) => merged.has(x))
				return
			}
		}
		selectedIds = selectedIds.includes(id)
			? selectedIds.filter((x) => x !== id)
			: [...selectedIds, id]
	}

	function toggleSelectAll() {
		selectedIds = allSelected ? [] : [...selectableIds]
	}

	// Home shows no archived draft, so archiving the one its composer holds moves Home on.
	function leaveArchivedHomeDraft(ids: string[]) {
		if (page.url.pathname === '/' && homeDraft.id && ids.includes(homeDraft.id)) {
			void leaveForHome()
		}
	}

	function handleBatchArchive() {
		const archive = !allSelectedArchived
		let skipped = 0
		for (const id of selectedIds) {
			const session = sessionState.sessions.find((s) => s.id === id)
			if (!session) continue
			// Unarchiving a session whose fork workspace is gone can't persist (the
			// putSession guard drops it) and reconcile would re-archive it, so the row
			// would silently revert. Same guard as the per-row menu.
			if (!archive && isUnavailableFork(session)) {
				skipped++
				continue
			}
			setSessionArchived(id, archive)
		}
		if (archive) leaveArchivedHomeDraft(selectedIds)
		exitSelectionMode()
		if (skipped > 0) {
			sendUserToast(
				`${skipped} session${skipped === 1 ? '' : 's'} kept archived: their forked workspace no longer exists`
			)
		}
	}

	// Batch delete never touches fork workspaces — same default as the single
	// delete, where the user has to tick the fork box explicitly.
	async function handleConfirmedBatchDelete() {
		const ids = selectedIds
		batchDeleteOpen = false
		if (ids.length === 0) return
		const active = activeRowId
		const wasActive = !!active && ids.includes(active)
		await withOpenSessionTeardown(async () => {
			for (const id of ids) removeSession(id)
			exitSelectionMode()
			if (wasActive) await leaveForHome()
		})
	}

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
		const wasActive = activeRowId === session.id
		await withOpenSessionTeardown(async () => {
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
			if (wasActive) await leaveForHome()
		})
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
				<Menu {createMenu} usePointerDownOutside submenuSafe closeFocus={railMenuCloseFocus}>
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
							<div class="py-1" role="none" data-asks-for-composer>
								<MenuItem
									class={menuItemBase}
									onClick={() => {
										newSessionFromRail = true
										void openNewSession()
									}}
									{item}
								>
									<Plus size={14} />
									New session
								</MenuItem>
							</div>
							<div class="py-1" role="none">
								<SessionFilterMenu
									{builders}
									bind:showArchived={showArchived.val}
									bind:lastActivityDays={lastActivityDays.val}
									bind:groupBy={groupBy.val}
									{archivedCount}
								/>
							</div>
							<div class="py-1" role="none">
								{#each displayGroups as group (group.key)}
									{#if group.showHeader}
										<div
											class="px-3 pt-1.5 pb-0.5 text-3xs text-tertiary truncate"
											role="none"
											title={group.label}
										>
											{group.label}
										</div>
									{/if}
									{#each group.sessions as session (session.id)}
										{@const runtime = getRuntime(session.id)}
										{@const status = runtime
											? getSessionChatStatus(runtime)
											: (getSessionChatPeek(session.id)?.status ?? 'idle')}
										{@const isSelected = session.id === activeRowId}
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
													<UnreadCountBadge count={unread} class="min-w-4 h-4 text-[10px]" />
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
		<!-- Selection mode reuses the options row (same h-8 slot) so the list below it
		     never moves. -->
		{#if selectionMode}
			<div class="flex flex-row items-center gap-1.5 h-8 px-2">
				<Checkbox
					checked={allSelected}
					indeterminate={selectedIds.length > 0 && !allSelected}
					onChange={toggleSelectAll}
					class="shrink-0 !w-4 !h-4 !p-0"
					title="Select all sessions"
				/>
				<Button
					unifiedSize="2xs"
					variant="subtle"
					on:click={toggleSelectAll}
					btnClasses="!text-xs !h-5 w-auto !px-0 text-secondary whitespace-nowrap"
				>
					Select all
				</Button>
				<span class="ml-auto text-2xs text-tertiary whitespace-nowrap">
					{selectedIds.length} selected
				</span>
				<Button
					unifiedSize="2xs"
					variant="subtle"
					btnClasses="!text-2xs !h-5 w-auto"
					onClick={exitSelectionMode}
				>
					Done
				</Button>
			</div>
		{:else}
			<!-- The submenu collapses to one row, so the picked cutoff would otherwise
			     only be visible after opening it. -->
			{#snippet lastActivityHint()}
				<span class="text-2xs text-tertiary whitespace-nowrap">
					{LAST_ACTIVITY_OPTIONS.find((o) => o.days === lastActivityDays.val)?.hint ?? ''}
				</span>
			{/snippet}
			{#snippet groupByHint()}
				<span class="text-2xs text-tertiary whitespace-nowrap">
					{GROUP_BY_OPTIONS.find((o) => o.value === groupBy.val)?.hint ?? ''}
				</span>
			{/snippet}
			{#snippet listOptions(size: 'sm' | 'md')}
				<DropdownV2
					fixedHeight={false}
					placement="bottom-end"
					enableFlyTransition
					items={[
						{
							displayName: archivedCount > 0 ? `Show archived (${archivedCount})` : 'Show archived',
							icon: Archive,
							toggle: showArchived.val,
							action: () => (showArchived.val = !showArchived.val)
						},
						{
							displayName: 'Last activity',
							icon: Clock,
							extra: lastActivityHint,
							submenuItems: LAST_ACTIVITY_OPTIONS.map((o) => ({
								displayName: o.label,
								selected: lastActivityDays.val === o.days,
								action: () => (lastActivityDays.val = o.days)
							}))
						},
						{
							displayName: 'Group by',
							icon: Rows3,
							extra: groupByHint,
							submenuItems: GROUP_BY_OPTIONS.map((o) => ({
								displayName: o.label,
								selected: groupBy.val === o.value,
								action: () => (groupBy.val = o.value)
							}))
						},
						{
							displayName: 'Edit sessions',
							icon: ListChecks,
							action: enterSelectionMode,
							separatorTop: true,
							// Selection is built from `visibleSessions`, which the tree render
							// path doesn't follow — it drops workspace-less sessions and hides
							// rows under a collapsed workspace, so "Select all" there would
							// reach rows that never showed a checkbox.
							hide: visibleSessions.length === 0 || workspaceTree
						}
					]}
				>
					{#snippet buttonReplacement()}
						<Button
							nonCaptureEvent
							unifiedSize={size}
							variant="subtle"
							iconOnly
							startIcon={{ icon: EllipsisVertical }}
							title="Session list options"
							aria-label="Session list options"
							btnClasses="text-secondary"
						/>
					{/snippet}
				</DropdownV2>
			{/snippet}
			{#if collapsible}
				<SidebarSectionHeader
					title="AI sessions"
					collapsed={sectionCollapsed.val}
					onToggle={() => (sectionCollapsed.val = !sectionCollapsed.val)}
					disabled={visibleSessions.length === 0}
				>
					{#snippet actions()}
						{@render listOptions('sm')}
						<Button
							unifiedSize="sm"
							variant="subtle"
							iconOnly
							startIcon={{ icon: Plus }}
							title="New session"
							aria-label="New session"
							data-asks-for-composer
							btnClasses="text-secondary"
							onClick={() => openNewSession()}
						/>
					{/snippet}
				</SidebarSectionHeader>
			{:else}
				<!-- Where the list owns the pane, a title would only repeat what the pane is. -->
				<div class="flex flex-row items-center gap-0.5 h-8 pr-0.5 justify-end">
					{@render listOptions('md')}
				</div>
			{/if}
		{/if}
		{#if !collapsible || !sectionCollapsed.val}
			<div
				bind:this={listRoot}
				transition:slide={{ duration: 180 }}
				class={twMerge(
					'flex flex-col gap-0.5',
					// Shift-clicking rows would otherwise paint a text selection across them.
					selectionMode ? 'select-none' : ''
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
					{@const status = runtime
						? getSessionChatStatus(runtime)
						: (getSessionChatPeek(session.id)?.status ?? 'idle')}
					{@const isSelected = session.id === activeRowId}
					{@const isEditing = editingId === session.id}
					{@const unread = unreadFor(session)}
					{@const draft = hasDraft(session)}
					{@const isChecked = selectedIds.includes(session.id)}
					<div
						class={twMerge(
							'flex flex-row group rounded',
							treeDepth === undefined ? 'items-center' : 'items-stretch',
							isSelected ? sidebarClasses.selectedBg : 'hover:bg-surface-hover',
							session.archived ? 'opacity-60' : '',
							// Grouped rows sit slightly in from their header; tree mode uses guide columns.
							treeDepth === undefined && indented ? 'pl-1' : ''
						)}
					>
						{#if treeDepth !== undefined}
							{#each Array(treeDepth) as _}{@render vline()}{/each}
						{/if}
						{#if selectionMode}
							<!-- Takes over the status-dot slot exactly (pl-2 + 16px, the row button's
							     own px-2 supplying the trailing gap), so entering selection mode
							     doesn't shift a single row label sideways. -->
							<span class="flex items-center pl-2">
								<Checkbox
									checked={isChecked}
									onChange={() => {
										toggleSelected(session.id, shiftHeldOnClick)
										shiftHeldOnClick = false
									}}
									onClick={(e) => (shiftHeldOnClick = e.shiftKey)}
									class="shrink-0 !w-4 !h-4 !p-0"
									title={session.summary ?? 'Untitled session'}
								/>
							</span>
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
								aria-selected={selectionMode ? isChecked : isSelected}
								onclick={(e) =>
									selectionMode ? toggleSelected(session.id, e.shiftKey) : activate(session)}
								class={twMerge(
									'flex flex-row items-center gap-2 text-left text-xs font-normal focus:outline-none flex-1 min-w-0 px-2 py-1',
									isSelected ? sidebarClasses.selectedText : 'text-secondary'
								)}
							>
								{#if treeDepth === undefined && !selectionMode}
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
										<UnreadCountBadge count={unread} class="min-w-4 h-4 text-[10px]" />
									</span>
								{/if}
							</button>
							<!-- Hidden rather than dropped while selecting: removing it would widen
							     every label and re-truncate the list. -->
							<div
								class={twMerge(
									'transition-opacity pr-0.5',
									selectionMode
										? 'invisible pointer-events-none'
										: 'opacity-0 group-hover:opacity-100 focus-within:opacity-100'
								)}
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
														action: () => {
															setSessionArchived(session.id, true)
															leaveArchivedHomeDraft([session.id])
														}
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
					{#each displayGroups as group, groupIdx (group.key)}
						{#if group.showHeader}
							{@const groupWsId = group.workspaceId}
							{@const groupWs = groupWsId
								? $userWorkspaces.find((w) => w.id === groupWsId)
								: undefined}
							<!-- A group header is the section title for the rows under it:
							     plain text, no workspace icon or chip. -->
							<div
								class={twMerge(
									'group flex flex-row items-center gap-1 pl-2 pr-0.5 pt-2 pb-1 min-w-0',
									// Space groups apart so their boundaries read clearly.
									groupIdx > 0 ? 'mt-4' : ''
								)}
								title={group.label}
							>
								<span class="text-secondary text-3xs truncate min-w-0">
									{group.label}
								</span>
								{#if groupWs?.is_dev_workspace}
									<DevWorkspaceBadge
										label={groupWs.dev_workspace_label}
										color={groupWs.color}
										fallbackColor="gray"
										class="text-3xs px-1 py-0 shrink-0"
									/>
								{/if}
								{#if groupWsId}
									<Button
										unifiedSize="xs"
										variant="subtle"
										iconOnly
										startIcon={{ icon: Plus }}
										on:click={() => createAndOpenIn(groupWsId)}
										title="New session in {group.label}"
										aria-label="New session in {group.label}"
										data-asks-for-composer
										wrapperClasses="ml-auto shrink-0"
										btnClasses="text-tertiary opacity-0 group-hover:opacity-100 focus-visible:opacity-100 transition-opacity"
									/>
								{/if}
							</div>
						{/if}
						{#each group.sessions as session (session.id)}
							{@render sessionRow(session, group.showHeader, undefined)}
						{/each}
					{/each}
				{/if}
			</div>
			{#if selectionMode}
				<!-- Pinned to the sidebar's bottom edge while the rows scroll under it; z-10 keeps it
				     above the sidebar's bottom fade. -->
				<div
					class="sticky bottom-0 z-10 flex flex-col gap-1 px-1 pt-1.5 pb-2 bg-[var(--sidebar-bg-light)] dark:bg-[var(--sidebar-bg-dark)]"
				>
					<Button
						unifiedSize="xs"
						variant="default"
						disabled={selectedIds.length === 0}
						startIcon={{ icon: allSelectedArchived ? ArchiveRestore : Archive }}
						onClick={handleBatchArchive}
					>
						{allSelectedArchived ? 'Unarchive' : 'Archive'}
					</Button>
					<Button
						unifiedSize="xs"
						variant="default"
						destructive
						disabled={selectedIds.length === 0}
						startIcon={{ icon: Trash2 }}
						onClick={() => (batchDeleteOpen = true)}
					>
						Delete
					</Button>
				</div>
			{/if}
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
				>{pendingDelete?.summary ?? 'Untitled session'}</span
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

<ConfirmationModal
	open={batchDeleteOpen}
	title="Delete sessions"
	confirmationText="Delete"
	onConfirmed={handleConfirmedBatchDelete}
	onCanceled={() => (batchDeleteOpen = false)}
>
	<div class="flex flex-col gap-3">
		<p>
			Delete <span class="font-medium text-primary"
				>{selectedIds.length} session{selectedIds.length === 1 ? '' : 's'}</span
			>? This cannot be undone.
		</p>
		{#if selectedForkCount > 0}
			<p class="text-xs text-tertiary">
				{selectedForkCount} of them {selectedForkCount === 1 ? 'has a' : 'have'} forked workspace{selectedForkCount ===
				1
					? ''
					: 's'}, which {selectedForkCount === 1 ? 'is' : 'are'} kept. Delete a session on its own to
				drop its fork too.
			</p>
		{/if}
	</div>
</ConfirmationModal>
