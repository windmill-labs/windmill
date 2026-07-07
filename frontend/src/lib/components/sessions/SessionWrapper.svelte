<script lang="ts">
	import { setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import { Button, NameIdTooltip } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'
	import { copilotInfo, loadCopilot } from '$lib/aiStore'
	import {
		Archive,
		ArchiveRestore,
		ArrowUpRight,
		EllipsisVertical,
		ExternalLink,
		PanelRightClose,
		PanelRightOpen,
		Pencil,
		Settings,
		Trash2
	} from 'lucide-svelte'
	import { type Item } from '$lib/utils'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import WorkspaceItemDrillPicker from '$lib/components/WorkspaceItemDrillPicker.svelte'
	import FlowEditorView from './FlowEditorView.svelte'
	import ScriptEditorView from './ScriptEditorView.svelte'
	import RawAppEditorView from './RawAppEditorView.svelte'
	import PipelineEditorView from './PipelineEditorView.svelte'
	import SessionWorkspaceBar from './SessionWorkspaceBar.svelte'
	import SessionChangesBar from './SessionChangesBar.svelte'
	import {
		createSession,
		deleteSessionsForWorkspace,
		getEffectiveWorkspaceId,
		moveSessionToNewFork,
		moveSessionToWorkspace,
		peekTransientDraftPrompt,
		queueTransientDraftPrompt,
		reconcileAfterWorkspaceChange,
		renameSession,
		selectSession,
		sessionState,
		setSessionArchived,
		setSessionTarget,
		syncWorkspaceTo,
		type SessionTarget
	} from './sessionState.svelte'
	import { editorWarmIds, getOrCreateRuntime, removeSession } from './sessionRuntime.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$app/paths'
	import { slide } from 'svelte/transition'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'

	// hideEditor: never mount the inline editor pane. Used by the sessions page,
	// where the edited item is shown in a live page preview (iframe) beside the
	// chat instead, so the wrapper contributes only its chat column.
	// headerInset: extra left padding on the chat header so it clears a floating
	// control (the collapsed-rail launcher) sitting at the screen's top-left.
	let {
		sessionId,
		hideEditor = false,
		headerInset = false
	}: {
		sessionId: string
		hideEditor?: boolean
		headerInset?: boolean
	} = $props()

	// LRU-warm sessions get their editor pane mounted; others render
	// chat-only. Reading from the reactive Set keeps SessionWrapper in
	// sync with promoteEditorWarm without an explicit prop round-trip
	// through the page route.
	const mountEditor = $derived(editorWarmIds.has(sessionId))

	// Parent keys by sessionId; this wrapper only mounts when the session exists.
	// Captured at script-init so we can synchronously bind context.
	const initialSession = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = initialSession ? getOrCreateRuntime(initialSession) : undefined

	if (runtime) {
		setContext<AIChatManager>('aiChatManager', runtime.manager)
	}

	// Reactive session reference (mutations to summary/target propagate via the $state proxy)
	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))

	// Seed the composer with the unsent prompt a reload preserved in the
	// transient draft slot (script-init: AIChatInput reads it once at mount).
	const restoredDraftPrompt = peekTransientDraftPrompt(sessionId)

	// The workspace the session acts on, shown in the header "Acting on" strip via the shared
	// WorkspaceScopeTrigger chip. `targetId` is also the workspace the chip's ellipsis menu targets.
	const acting = $derived.by(() => {
		const wsId = session ? getEffectiveWorkspaceId(session) : undefined
		if (!wsId) return undefined
		const name = $userWorkspaces.find((w) => w.id === wsId)?.name ?? wsId
		return { targetId: wsId, name }
	})

	// Ellipsis menu on the "Acting on" chip. Both entries are real links (so
	// modifier/middle clicks open a new tab); the `workspace` query param
	// points the navigation at the acting workspace — the layout applies it on
	// both full loads and client-side query changes. The trailing external-link
	// glyphs make the leave-the-session navigation explicit.
	const actingMenu = $derived<Item[]>([
		{
			displayName: 'Workspace settings',
			icon: Settings,
			href: `${base}/workspace_settings?workspace=${acting?.targetId ?? ''}`,
			extra: externalLinkHint
		},
		{
			displayName: 'Go to this workspace',
			icon: ArrowUpRight,
			href: `${base}/?workspace=${acting?.targetId ?? ''}`,
			extra: externalLinkHint
		}
	])

	// Load copilot config (models, providers) for the workspace the session acts
	// on, not the navigation workspace — a session deliberately leaves
	// $workspaceStore on the nav workspace, so keying off it would pick the model
	// and provider from the wrong workspace's AI config for a fork-scoped session.
	// Only the active session may write this: copilotInfo/copilotSessionModel are
	// global, and warm background wrappers (each in their own workspace) would
	// otherwise race to clobber the active chat's model config.
	$effect(() => {
		if (sessionState.currentSessionId !== sessionId) return
		const ws = acting?.targetId ?? $workspaceStore
		if (ws) {
			loadCopilot(ws)
		}
	})

	let summaryInput: EditableInput | undefined = $state(undefined)

	// Drop the user on a fresh new-session page. Used after archiving or
	// deleting the open session: the session they were on is no longer
	// usable, and routing to a sibling would feel arbitrary.
	async function resetToNewSession() {
		const fresh = createSession()
		selectSession(fresh.id)
		// The page derives the visible session from the `session_name` query, not
		// currentSessionId — navigate so the URL leaves the deleted/archived session
		// (else it renders the not-found state or stays on the archived one).
		await goto(`/sessions?session_name=${encodeURIComponent(fresh.name)}`)
	}

	// If the session targets a forked workspace that's still accessible,
	// offer to delete / archive the fork alongside the session — otherwise
	// the fork lingers as an orphan whose only purpose was this session.
	const sessionForkId = $derived.by(() => {
		const wsId = session?.workspace_id
		if (!wsId) return undefined
		const ws = $userWorkspaces.find((w) => w.id === wsId)
		// Don't offer the option if the fork is gone/not user-accessible or isn't a fork (prefix OR
		// parent, so an orphaned wm-fork- fork still qualifies).
		if (!ws || !workspaceIsFork(wsId, $userWorkspaces)) return undefined
		// A persistent dev workspace is not an ephemeral session fork — never offer to delete it.
		if (ws.is_dev_workspace) return undefined
		return wsId
	})

	let deleteConfirmOpen = $state(false)
	let deleteAlsoFork = $state(false)
	let archiveConfirmOpen = $state(false)
	let archiveAlsoFork = $state(false)

	async function handleConfirmedDelete() {
		deleteConfirmOpen = false
		if (!session) return
		const forkToDelete = deleteAlsoFork ? sessionForkId : undefined
		// Capture the fork's parent before the workspace list is refreshed
		// below — afterwards the fork is gone from $userWorkspaces.
		const forkParentId = forkToDelete
			? $userWorkspaces.find((w) => w.id === forkToDelete)?.parent_workspace_id
			: undefined
		deleteAlsoFork = false
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
		// so the new session (created below) opens against a live workspace
		// instead of the one we just removed.
		if (forkToDelete && forkParentId && $workspaceStore === forkToDelete) {
			syncWorkspaceTo(forkParentId)
		}
		await resetToNewSession()
	}

	async function handleConfirmedArchive() {
		archiveConfirmOpen = false
		if (!session) return
		const forkToArchive = archiveAlsoFork ? sessionForkId : undefined
		archiveAlsoFork = false
		setSessionArchived(session.id, true)
		if (forkToArchive) {
			try {
				await WorkspaceService.archiveWorkspace({ workspace: forkToArchive })
				sendUserToast(`Archived forked workspace ${forkToArchive}`)
				await reconcileAfterWorkspaceChange()
			} catch (e: any) {
				sendUserToast(`Failed to archive fork ${forkToArchive}: ${e?.body ?? e}`, true)
			}
		}
		await resetToNewSession()
	}

	// Kept for the "Archive" entry that doesn't go through the confirmation
	// modal — when the session isn't in a fork, no extra question to ask.
	async function archiveAndReset() {
		if (!session) return
		// If the session is in a fork, route through the confirm modal so the
		// user can opt into archiving the fork. Otherwise skip the modal.
		if (sessionForkId) {
			archiveAlsoFork = false
			archiveConfirmOpen = true
			return
		}
		setSessionArchived(session.id, true)
		await resetToNewSession()
	}

	// Workspace bar is shown only before the session sends its first user
	// message — after that the session's workspace is immutable. The
	// commit itself happens in `AIChatManager.beforeSend` (wired in
	// `createRuntime`) so it fires exactly once at send-time. A reactive
	// commit here would retry forever on backend failures (e.g. fork-id
	// collision after a previously-successful create whose response was
	// dropped) — restoration self-heal lives in `initRuntime` instead.
	const hasFirstUserMessage = $derived(
		runtime?.manager.displayMessages.some((m) => m.role === 'user') ?? false
	)

	// Effective workspace for routing editor views — committed if set,
	// otherwise the pending pick, otherwise the current active workspace.
	const effectiveWorkspaceId = $derived(
		session ? (getEffectiveWorkspaceId(session) ?? $workspaceStore ?? '') : ''
	)

	// Core mutation: assign a target via the canonical setter, then re-open
	// the editor pane. Shared by every code path that swaps the session's
	// editor target (drill picker, fork-bar dropdown, …).
	function applyEditorTarget(target: SessionTarget, summary?: string) {
		if (!session) return
		setSessionTarget(session.id, target, summary)
		// Picking a target also re-opens the editor pane (the user just chose
		// what to view).
		editorVisible = true
	}

	function pickEditorTarget(item: WorkspaceItem) {
		// Legacy drag-and-drop apps aren't hosted in the session preview pane —
		// open them in the standalone app editor instead. Only code-based raw
		// apps (item.raw_app) are previewable here.
		if (item.kind === 'app' && !item.raw_app) {
			goto(`/apps/edit/${item.path}?workspace=${effectiveWorkspaceId}`)
			return
		}
		// WorkspaceItem.kind is 'flow'|'script'|'app'; any 'app' reaching here is
		// a raw app. The diff-API uses 'raw_app' as its kind so we align
		// SessionTarget on the same canonical string.
		const kind: SessionTarget['kind'] = item.kind === 'app' ? 'raw_app' : item.kind
		applyEditorTarget({ kind, path: item.path }, item.summary)
	}

	// Editor pane visibility. Toggling this just hides/shows the pane via CSS
	// — the editor stays mounted, so re-opening doesn't pay a remount cost
	// and xy-flow / Monaco keep their viewport state.
	let editorVisible = $state(true)

	// Focus the chat input whenever this session is the active one.
	// The textarea is disabled until copilotInfo loads (otherwise focus is
	// a silent no-op), so we wait for that too. Triggers on initial mount,
	// warm-session switch via the picker, and the moment copilot finishes
	// loading.
	let aiChat: AIChat | undefined = $state(undefined)
	$effect(() => {
		if (sessionState.currentSessionId !== sessionId) return
		if (!aiChat) return
		if (!$copilotInfo.enabled) return
		const chat = aiChat
		setTimeout(() => chat.focusInput(), 0)
	})

	// True when the session committed to a workspace that's no longer in
	// the user's list (deleted / archived / access revoked). The chat is
	// disabled and SessionChangesBar shows a move/discard banner.
	const isUnavailable = $derived(
		!!session?.workspace_id && !$userWorkspaces.find((w) => w.id === session!.workspace_id)
	)

	async function moveAndActivate(targetWorkspaceId: string) {
		if (!session) return
		moveSessionToWorkspace(session.id, targetWorkspaceId)
		// Point the app at the moved-to workspace too. Without this the global
		// workspace stays on the old (now-unavailable) one, so scope/editor keep
		// resolving against a dead workspace — mirrors what moveSessionToNewFork
		// does internally and handleConfirmedDelete does explicitly.
		syncWorkspaceTo(targetWorkspaceId)
	}

	async function createForkAndMove(fork: {
		parent_workspace_id: string
		id: string
		name: string
	}) {
		if (!session) return
		await moveSessionToNewFork(session.id, fork)
	}
</script>

{#snippet externalLinkHint()}
	<ExternalLink size={12} class="shrink-0 text-tertiary" />
{/snippet}

{#if !session || !runtime}
	<div class="p-8 text-secondary text-sm">Session not found</div>
{:else}
	{@const hasTarget =
		session.target?.kind === 'flow' ||
		session.target?.kind === 'script' ||
		session.target?.kind === 'raw_app' ||
		session.target?.kind === 'pipeline'}
	{@const hasEditor = mountEditor && hasTarget && editorVisible && !hideEditor}

	{#snippet inputPreface()}
		{#if !hasFirstUserMessage}
			<SessionWorkspaceBar {session} />
		{/if}
		<!-- gap-1 (4px) spaces the archived banner and the changes bar when both
		     are visible. Each renders a single in-flow root (or nothing); the diff
		     drawer is position:fixed, so it doesn't count as a flex item — no stray
		     gap when only one shows. -->
		<div class="flex flex-col gap-1">
			{#if session.archived && !isUnavailable}
				<!-- Unarchive is only meaningful when the workspace is still live:
				     putSession refuses to resurrect a session whose workspace is gone,
				     and reconcile would re-archive a workspace-archived one anyway. When
				     the workspace is unavailable the SessionChangesBar below shows the
				     move/discard banner instead (its actions are the real recovery path). -->
				<div
					class="flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md bg-surface-tertiary"
				>
					<div class="flex flex-row items-center gap-2 min-w-0">
						<Archive class="w-4 h-4 shrink-0 text-tertiary" />
						<span class="text-primary font-medium">This session is archived</span>
					</div>
					<Button
						variant="default"
						unifiedSize="sm"
						startIcon={{ icon: ArchiveRestore }}
						onclick={() => setSessionArchived(session.id, false)}
					>
						Unarchive
					</Button>
				</div>
			{/if}
			<SessionChangesBar
				{session}
				onMove={(workspaceId) => moveAndActivate(workspaceId)}
				onCreateForkAndMove={(fork) => createForkAndMove(fork)}
				onArchive={() => archiveAndReset()}
				onDelete={() => (deleteConfirmOpen = true)}
			/>
		</div>
	{/snippet}

	<!-- Override the chat's default keyboard-shortcut hint with nothing —
	     sessions have their own empty-state affordances above. -->
	{#snippet sessionEmptyHint()}{/snippet}

	<!-- Undefined pane sizes (not an explicit `size`): Splitpanes auto-distributes —
	     a lone chat pane fills 100%, and when the editor pane mounts the two split
	     50/50. A reactive `size={hasEditor ? 50 : 100}` here instead races the
	     sibling pane appearing on reload → "Could not resize panes due to constraints"
	     and a wrong split. -->
	<div class="flex-1 min-h-0 flex flex-col" use:splitterPointerCapture>
		<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
			<Pane minSize={25} class="flex flex-col min-h-0 pb-2">
				<header
					class="flex flex-row items-center gap-1 {headerInset
						? 'pl-11'
						: 'pl-4'} pr-4 py-2 shrink-0"
				>
					<EditableInput
						bind:this={summaryInput}
						value={session.summary ?? ''}
						placeholder="Untitled session"
						onSave={(v) => renameSession(session.id, v)}
						class="text-sm font-semibold"
						inputClass="!text-sm !font-semibold"
					/>
					<DropdownV2
						fixedHeight={false}
						placement="bottom-start"
						enableFlyTransition
						items={[
							{
								displayName: 'Rename',
								icon: Pencil,
								action: () => summaryInput?.edit()
							},
							...(session.archived
								? // No Unarchive when the workspace is gone — it can't persist
									// (putSession guard) and reconcile would re-archive it.
									isUnavailable
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
											action: () => archiveAndReset()
										}
									]),
							{
								displayName: 'Delete',
								icon: Trash2,
								type: 'delete',
								action: () => (deleteConfirmOpen = true)
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
					{#if acting && hasFirstUserMessage}
						<!-- "Acting on" context: workspace root (avatar + name) and, when the
					     session runs in a fork, the fork name (accent pill) — with a button
					     to jump into that workspace. Compact; sits right after the title.
					     Hidden until the session has started — a new (un-sent) session shows
					     the "Run in" picker (SessionWorkspaceBar) instead. -->
						<div class="flex items-center gap-1 min-w-0 text-2xs text-tertiary">
							<span class="shrink-0">Acting on</span>
							<!-- Hover reveals the workspace name + id + copy button (shared
							     NameIdTooltip, same as the sidebar family picker), so the chip
							     carries the copy affordance without an inline button. -->
							<NameIdTooltip name={acting.name} id={acting.targetId}>
								<WorkspaceScopeTrigger
									workspaceId={acting.targetId}
									showChevron={false}
									interactive={false}
									disableTitle
									class="max-w-[16rem]"
									menuItems={actingMenu}
								/>
							</NameIdTooltip>
						</div>
					{/if}
					{#if !hideEditor && !session.target && hasFirstUserMessage}
						<!-- Drill-picker for sessions that have started but haven't
				     picked an editor target yet. Hidden on fresh sessions
				     (no messages yet) — the workspace bar is the only
				     header affordance during the empty state. -->
						<div class="ml-auto">
							<Popover
								placement="bottom-end"
								usePointerDownOutside
								disableFocusTrap
								enableFlyTransition
								class="inline-flex"
							>
								{#snippet trigger()}
									<Button variant="default" unifiedSize="xs" startIcon={{ icon: PanelRightOpen }}>
										Open editor
									</Button>
								{/snippet}
								{#snippet content()}
									<WorkspaceItemDrillPicker
										onPick={(item: WorkspaceItem) => pickEditorTarget(item)}
									/>
								{/snippet}
							</Popover>
						</div>
					{:else if !hideEditor && hasTarget && mountEditor && !editorVisible}
						<div class="ml-auto">
							<Button
								variant="subtle"
								unifiedSize="xs"
								startIcon={{ icon: PanelRightOpen }}
								onclick={() => (editorVisible = true)}
							>
								Show editor
							</Button>
						</div>
					{:else if hasEditor}
						<div class="ml-auto flex flex-row items-center gap-1">
							<button
								type="button"
								onclick={() => (editorVisible = false)}
								title="Close editor"
								aria-label="Close editor"
								class="inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
							>
								<PanelRightClose size={14} />
							</button>
						</div>
					{/if}
				</header>
				<div class="flex-1 min-h-0 w-full flex flex-col {hasFirstUserMessage ? '' : 'pt-8'}">
					<AIChat
						bind:this={aiChat}
						hideHeader
						hideModeSelector
						wideLayout
						initialInstructions={restoredDraftPrompt}
						onDraftChange={(text) => queueTransientDraftPrompt(sessionId, text)}
						forceDisabled={isUnavailable || !!session.archived}
						forceDisabledMessage={isUnavailable
							? 'This session is linked to a workspace that no longer exists. Move it or discard it from the banner above to keep working.'
							: session.archived
								? 'This session is archived. Unarchive it from the banner above to keep working.'
								: ''}
						emptyHint={sessionEmptyHint}
						{inputPreface}
					/>
				</div>
			</Pane>
			{#if hasEditor && session.target}
				<Pane minSize={30} class="flex flex-col min-h-0 p-2 pl-0">
					<div
						transition:slide={{ axis: 'x', duration: 200 }}
						class="flex flex-col flex-1 min-h-0 rounded-md border border-light overflow-hidden relative"
					>
						{#if session.target.kind === 'flow'}
							<FlowEditorView
								{runtime}
								path={session.target.path}
								workspaceId={effectiveWorkspaceId}
								onNavigate={pickEditorTarget}
								isActiveSession={sessionState.currentSessionId === sessionId}
							/>
						{:else if session.target.kind === 'script'}
							<ScriptEditorView
								{runtime}
								path={session.target.path}
								workspaceId={effectiveWorkspaceId}
								onNavigate={pickEditorTarget}
								initialTestPanelCollapsed
								isActiveSession={sessionState.currentSessionId === sessionId}
							/>
						{:else if session.target.kind === 'raw_app'}
							<RawAppEditorView
								{runtime}
								path={session.target.path}
								workspaceId={effectiveWorkspaceId}
								onNavigate={pickEditorTarget}
								isActiveSession={sessionState.currentSessionId === sessionId}
							/>
						{:else if session.target.kind === 'pipeline'}
							<PipelineEditorView
								{runtime}
								path={session.target.path}
								workspaceId={effectiveWorkspaceId}
								isActiveSession={sessionState.currentSessionId === sessionId}
							/>
						{/if}
					</div>
				</Pane>
			{/if}
		</Splitpanes>
	</div>

	<ConfirmationModal
		open={deleteConfirmOpen}
		title="Delete session"
		confirmationText="Delete"
		onConfirmed={handleConfirmedDelete}
		onCanceled={() => {
			deleteConfirmOpen = false
			deleteAlsoFork = false
		}}
	>
		<div class="flex flex-col gap-3">
			<p>
				Delete session <span class="font-medium text-primary"
					>{session?.summary ?? session?.name}</span
				>? This cannot be undone.
			</p>
			{#if sessionForkId}
				<div class="flex items-start gap-2 border rounded-md p-3 bg-surface-secondary">
					<Toggle size="xs" bind:checked={deleteAlsoFork} />
					<div class="flex flex-col">
						<span class="text-xs font-medium text-primary"
							>Also delete forked workspace <span class="font-mono">{sessionForkId}</span></span
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
		open={archiveConfirmOpen}
		title="Archive session"
		confirmationText="Archive"
		onConfirmed={handleConfirmedArchive}
		onCanceled={() => {
			archiveConfirmOpen = false
			archiveAlsoFork = false
		}}
	>
		<div class="flex flex-col gap-3">
			<p>
				Archive session <span class="font-medium text-primary"
					>{session?.summary ?? session?.name}</span
				>? You can restore it later from the archived list.
			</p>
			{#if sessionForkId}
				<div class="flex items-start gap-2 border rounded-md p-3 bg-surface-secondary">
					<Toggle size="xs" bind:checked={archiveAlsoFork} />
					<div class="flex flex-col">
						<span class="text-xs font-medium text-primary"
							>Also archive forked workspace <span class="font-mono">{sessionForkId}</span></span
						>
						<span class="text-3xs text-tertiary"
							>Archived workspaces can be unarchived later from instance settings.</span
						>
					</div>
				</div>
			{/if}
		</div>
	</ConfirmationModal>
{/if}

<style>
	/* Invisible-but-draggable splitter: a real (layout-occupying) gutter, wide
	   enough to grab. No overlap tricks — the zone can't cover the left pane's
	   scrollbar or the right pane's edge. */
	:global(.splitpanes--vertical.splitter-hidden) > :global(.splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
		width: 10px !important;
	}
</style>
