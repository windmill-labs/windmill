<script module lang="ts">
	import { preloadComponent } from '$lib/utils/preloadComponent.svelte'

	// AIChat's code is most of this page's: loaded beside it rather than as part of it, so the
	// page and its ComposerPlaceholder show without waiting on it.
	const aiChatComponent = preloadComponent(
		() => import('$lib/components/copilot/chat/AIChat.svelte')
	)
</script>

<script lang="ts">
	import { onDestroy, setContext, untrack } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import ComposerPlaceholder from '$lib/components/copilot/chat/ComposerPlaceholder.svelte'
	import SessionGreeting from '$lib/components/copilot/chat/starter/SessionGreeting.svelte'
	import ComposerSuggestions from '$lib/components/copilot/chat/starter/ComposerSuggestions.svelte'
	import {
		globalComposerSuggestion,
		STARTER_PLACEHOLDER,
		type ComposerSelection
	} from '$lib/components/copilot/chat/composerBox'
	import { useSessionPermissions } from './sessionPermissions.svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { Button } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import { DraftService, WorkspaceService, type DraftItemRef } from '$lib/gen'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import SessionMoveModal from './SessionMoveModal.svelte'
	import { flushPendingSaves, moveSessionToFork, type MoveTarget } from './moveSessionToFork'
	import { maskHasDraftRow } from './modifiedItemsMask'
	import { sendUserToast } from '$lib/toast'
	import Toggle from '$lib/components/Toggle.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import {
		Archive,
		ArchiveRestore,
		EllipsisVertical,
		GitFork,
		MessageSquareDashed,
		Pencil,
		Plus,
		Trash2,
		X
	} from 'lucide-svelte'
	import SessionWorkspaceBar from './SessionWorkspaceBar.svelte'
	import { arrivingFromHomeSend } from './homeSessionsTransition.svelte'
	import SessionChangesBar from './SessionChangesBar.svelte'
	import SessionShareButton from './SessionShareButton.svelte'
	import { clearSessionRecovered, isSessionRecovered } from './sessionRecoveryNotice.svelte'
	import {
		composerFocusRequest,
		createSession,
		deleteSessionsForWorkspace,
		getEffectiveWorkspaceId,
		moveSessionToNewFork,
		moveSessionToWorkspace,
		getSessionDraftPrompt,
		setSessionDraftPrompt,
		getSessionDraftAttachments,
		setSessionDraftAttachments,
		peekSessionAutoSend,
		takeSessionAutoSend,
		reconcileAfterWorkspaceChange,
		renameSession,
		selectSession,
		sessionPageHref,
		sessionState,
		setSessionArchived,
		syncWorkspaceTo,
		withOpenSessionTeardown
	} from './sessionState.svelte'
	import { getOrCreateRuntime, removeSession } from './sessionRuntime.svelte'
	import { goto } from '$lib/navigation'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'
	import { StarterPrompts } from '$lib/components/copilot/chat/starter/starterPrompts.svelte'

	let {
		sessionId,
		headerInPage = false,
		covered = false
	}: {
		sessionId: string
		/** True for the session the page is showing: its header rides the page header bar. */
		headerInPage?: boolean
		/** True while the page holds this wrapper inert under an overlay, where nothing takes focus. */
		covered?: boolean
	} = $props()

	// Parent keys by sessionId; this wrapper only mounts when the session exists.
	// Captured at script-init so we can synchronously bind context.
	const initialSession = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = initialSession ? getOrCreateRuntime(initialSession) : undefined

	if (runtime) {
		setContext<AIChatManager>('aiChatManager', runtime.manager)
	}

	// Reactive session reference (mutations to summary/target propagate via the $state proxy)
	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))

	// Only a session that has never sent opens as the starter block. Read off the record, not
	// the transcript: a cold session's history is still loading from storage when it mounts,
	// and an empty transcript would show the greeting and then glide the composer away.
	// The first send commits the workspace, so this flips only after the transcript has.
	const opensSession = $derived(!session?.workspace_id)
	const starterPrompts = new StarterPrompts(() =>
		opensSession ? (actingWorkspaceId ?? $workspaceStore ?? undefined) : undefined
	)

	// Seed the composer with the unsent prompt a reload preserved on the session
	// record (script-init: AIChatInput reads it once at mount).
	const restoredDraftPrompt = getSessionDraftPrompt(sessionId)

	// Whether the prompt read above is already on its way, so the composer starts
	// empty instead of briefly showing text the effect below is about to send.
	// Both halves of the claim's own condition are required, not just freshness:
	// an intent this wrapper will not claim — stale, or armed on a session the
	// page is only keeping warm in the background — still gets a composer, whose
	// mount-time empty draft would otherwise erase the prompt from the record.
	const armedAtInit = sessionState.currentSessionId === sessionId && peekSessionAutoSend(sessionId)

	// The composer's text before AIChat mounts, typed into the placeholder, and then the real
	// composer's initial draft.
	let draftText = $state(armedAtInit ? '' : (restoredDraftPrompt ?? ''))
	// What the first focus after the swap does: put the caret where the placeholder had it, or,
	// when null, leave focus where the user moved it while the chat loaded. Undefined when the
	// placeholder never had focus.
	let handoff: ComposerSelection | null | undefined = undefined
	// The swap waits for an IME composition to end: the new field would not carry it over.
	let placeholderComposing = $state(false)
	// The placeholder's text counts as unsent input, like the real composer's draft, so the page
	// does not evict this session's chat (and the text with it) while it waits for the chat code.
	const PLACEHOLDER_DRAFT_KEY = 'composer-placeholder'
	function setPlaceholderText(text: string) {
		draftText = text
		setSessionDraftPrompt(sessionId, text)
		runtime?.manager.setComposerHasDraft(PLACEHOLDER_DRAFT_KEY, text.trim() !== '')
	}

	// Drawn once, and shared by the placeholder and the real composer so the text survives the swap.
	const suggestionPick = Math.random()
	const permissions = useSessionPermissions(() => runtime?.manager.operatingWorkspace)
	const composerPlaceholder = $derived(
		globalComposerSuggestion(suggestionPick, permissions.canOpenEditor)
	)

	// A hand-off whose click already stated the intent asks for its prompt to be
	// sent, not parked. Reactive rather than mount-time: `createSession` reuses an
	// untouched blank session and the sessions page keys wrappers by id, so a
	// hand-off fired from `/sessions` can arm the session already on screen —
	// whose wrapper never re-initializes, leaving an init-time claim to never run
	// and the prompt to never be sent. Claimed only for the session the user
	// landed on; a background wrapper would fire the turn off-screen.
	$effect(() => {
		if (sessionState.currentSessionId !== sessionId) return
		if (!session?.autoSendDraftAt || !runtime) return
		untrack(() => {
			const prompt = getSessionDraftPrompt(sessionId)
			// takeSessionAutoSend clears the intent whether or not it is still
			// fresh, so this cannot re-enter on the next dependency change.
			if (!takeSessionAutoSend(sessionId) || !prompt) return
			// sendOrQueue, not sendRequest: a reused session may already be mid-turn.
			// The runtime's beforeSend awaits loadCopilot for the committed
			// workspace, so this cannot race the model config on a cold session.
			runtime.manager.sendOrQueue(prompt)
		})
	})

	// The workspace the session acts on. The page header's breadcrumb scopes itself to it, so a
	// session running in a fork reads as that fork rather than as the navigation workspace.
	const actingWorkspaceId = $derived(session ? getEffectiveWorkspaceId(session) : undefined)

	// Load copilot config (models, providers) for the workspace the session acts
	// on, not the navigation workspace — a session deliberately leaves
	// $workspaceStore on the nav workspace, so keying off it would pick the model
	// and provider from the wrong workspace's AI config for a fork-scoped session.
	// Only the active session may write this: copilotInfo/copilotSessionModel are
	// global, and warm background wrappers (each in their own workspace) would
	// otherwise race to clobber the active chat's model config.
	$effect(() => {
		if (sessionState.currentSessionId !== sessionId) return
		const ws = actingWorkspaceId ?? $workspaceStore
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
		// The page derives the visible session from the `session` query, not
		// currentSessionId: navigate so the URL leaves the deleted/archived session,
		// which would otherwise fall through to recovery or stay on the archived one.
		await goto(sessionPageHref(fresh.id))
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
		await withOpenSessionTeardown(async () => {
			removeSession(sessionId)
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
		})
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

	// Gated on the first message, not on typing: the notice sits directly above a
	// vertically centred composer, so dropping it mid-sentence would jump the
	// field out from under the user's cursor.
	const showRecoveryNotice = $derived(isSessionRecovered(sessionId) && !hasFirstUserMessage)

	// The notice explains one arrival, and the arrival is exactly "this page is
	// open, on this session, nothing sent yet". Spend the marker the moment any of
	// those three breaks, so a later return can never replay it — masking it behind
	// the gate above instead leaves it to resurface. Dismiss is the user's own exit.
	$effect(() => {
		if (hasFirstUserMessage || sessionState.currentSessionId !== sessionId) {
			clearSessionRecovered(sessionId)
		}
	})
	onDestroy(() => clearSessionRecovered(sessionId))

	// Focus the chat input whenever this session is the active one.
	// The textarea is disabled until copilotInfo loads (otherwise focus is
	// a silent no-op), so we wait for that too. Triggers on initial mount,
	// warm-session switch via the picker, and the moment copilot finishes
	// loading.
	let aiChat: AIChat | undefined = $state(undefined)
	let seenFocusNonce = untrack(() => composerFocusRequest.nonce)
	$effect(() => {
		// Focus the composer when this session becomes active, or on an explicit
		// focus request — the latter covers `+` reusing the untouched draft you're
		// already viewing, where currentSessionId doesn't change so activation alone
		// wouldn't re-run this.
		const focusNonce = composerFocusRequest.nonce
		if (focusNonce !== seenFocusNonce) {
			// An explicit request overrides the user having left the placeholder before it.
			seenFocusNonce = focusNonce
			handoff = undefined
		}
		if (sessionState.currentSessionId !== sessionId) {
			// The user left for another session: coming back is an activation, which focuses.
			handoff = undefined
			return
		}
		if (!aiChat) return
		if (!$copilotInfo.enabled) return
		const chat = aiChat
		const selection = handoff
		handoff = undefined
		if (selection === null) return
		setTimeout(() => chat.focusInput(selection), 0)
	})

	// The real composer is disabled until the copilot config of the workspace it acts on is in,
	// so the active session keeps the placeholder, which takes typing, until then. Latched: a
	// later config reload must not swap a mounted chat, and its transcript, back out.
	let chatShown = false
	const showChat = $derived.by(() => {
		chatShown ||=
			!!aiChatComponent.current &&
			!placeholderComposing &&
			(sessionState.currentSessionId !== sessionId ||
				$copilotWorkspace === (actingWorkspaceId ?? $workspaceStore))
		return chatShown
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

	// Moving mid-turn would let a tool still running write its draft into the workspace
	// being left, after the move copied or removed the others.
	const canMoveToFork = $derived(
		!!runtime &&
			!isUnavailable &&
			!session?.archived &&
			!runtime.manager.loading &&
			!runtime.manager.sendInFlight &&
			!runtime.manager.sendPending &&
			!runtime.manager.runHeldElsewhere
	)

	type MoveRequest = {
		open: boolean
		parent: string
		parentName: string
		target: MoveTarget
		targetName: string
		items: (DraftItemRef & { label: string })[]
		conflicts: DraftItemRef[]
		error?: string
	}
	let movePickerOpen = $state(false)
	let moveRequest = $state<MoveRequest | undefined>(undefined)
	let moveRemoveFromParent = $state(false)
	let moving = $state(false)

	// Every chat of the session counts, not only the one on screen: a user who opened a new
	// chat inside the session still expects the earlier edits to come along.
	function sessionMask(): Set<string> {
		const m = runtime?.manager
		if (!m) return new Set()
		return new Set([
			...m.historyManager.getAllSavedChats().flatMap((c) => c.modifiedItems ?? []),
			...(m.modifiedItems ?? [])
		])
	}

	async function startMove(target: MoveTarget) {
		const parent = actingWorkspaceId
		if (!session || !parent || !canMoveToFork) return
		if (target.kind === 'existing' && target.id === parent) return
		const mask = sessionMask()
		let items: MoveRequest['items']
		let conflicts: DraftItemRef[] = []
		// The list comes from the server, so an edit still debounced would be left out of it.
		const unsaved = await flushPendingSaves(parent)
		if (unsaved) {
			sendUserToast(unsaved, true)
			return
		}
		try {
			// Only the user's own rows move; a legacy workspace-level draft is nobody's.
			items = (await DraftService.listDrafts({ workspace: parent }))
				.filter((r) => r.mine && !r.legacy_draft && maskHasDraftRow(mask, r))
				.map((r) => ({ kind: r.kind, path: r.path, label: r.draft_path ?? r.path }))
			if (target.kind === 'existing' && items.length > 0) {
				const inFork = new Set(
					(await DraftService.listDrafts({ workspace: target.id }))
						.filter((r) => r.mine && !r.legacy_draft)
						.map((r) => `${r.kind}:${r.path}`)
				)
				conflicts = items
					.filter((i) => inFork.has(`${i.kind}:${i.path}`))
					.map(({ kind, path }) => ({ kind, path }))
			}
		} catch (e: any) {
			sendUserToast(`Could not list the session's drafts: ${e?.body ?? e?.message ?? e}`, true)
			return
		}
		moveRemoveFromParent = false
		moveRequest = {
			open: items.length > 0,
			parent,
			parentName: $userWorkspaces.find((w) => w.id === parent)?.name ?? parent,
			target,
			targetName:
				target.kind === 'new'
					? target.fork.name
					: ($userWorkspaces.find((w) => w.id === target.id)?.name ?? target.id),
			items,
			conflicts
		}
		if (items.length === 0) await confirmMove()
	}

	async function confirmMove() {
		const req = moveRequest
		if (!session || !req || moving) return
		moving = true
		const releaseAutoResume = runtime?.manager.holdAutoResume()
		try {
			const res = await moveSessionToFork(session.id, req.parent, req.target, {
				items: req.items.map(({ kind, path }) => ({ kind, path })),
				removeFromParent: moveRemoveFromParent,
				isBusy: () => !canMoveToFork
			})
			if (!res.ok) {
				if (!req.open) {
					sendUserToast(res.error, true)
					moveRequest = undefined
				} else {
					moveRequest = { ...req, error: res.error, conflicts: res.conflicts ?? [] }
				}
				return
			}
			moveRequest = undefined
			void runtime?.manager
				.queueModelNote(
					`This session moved from workspace "${req.parent}" to its fork "${res.forkId}": every further tool call acts on "${res.forkId}". ` +
						(res.copied > 0
							? `${res.copied} draft(s) of this session were copied there` +
								(res.removed > 0 ? ` and removed from "${req.parent}". ` : '. ')
							: '') +
						`Jobs from earlier turns ran in "${req.parent}".`
				)
				.catch((e) => console.error('Failed to persist the move note', e))
			runtime?.manager.rebuildGlobalSystemMessage()
			sendUserToast(`Session moved to ${req.targetName}`)
			if (res.warning) sendUserToast(res.warning, true)
		} finally {
			moving = false
			releaseAutoResume?.()
		}
	}

	// Chrome shared by the banners stacked above the composer, so restyling one
	// can't leave the others behind. Each supplies its own background: two bg-*
	// utilities on one element resolve by stylesheet order, not attribute order.
	const bannerClass =
		'flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md'
</script>

{#if !session || !runtime}
	<div class="p-8 text-secondary text-sm">Session not found</div>
{:else}
	<!-- A sent session whose transcript is still loading (or came back empty) shows nothing
	     rather than the docked copilot's keyboard-shortcut hint, which means nothing here. -->
	{#snippet sessionEmptyHint()}{/snippet}

	{#snippet inputPreface()}
		{#if showRecoveryNotice}
			<!-- mb-6, not the banner column's mb-1: at that gap it reads as one more
			     row of composer chrome rather than as a message about the session. -->
			<!-- Tinted, where the other banners are white: this one has to be spotted
			     without being looked for, above a composer the user is already typing in. -->
			<!-- Announced: the composer takes focus the moment this appears, so without
			     it a screen reader lands in an empty field with no account of the swap. -->
			<div role="status" class="{bannerClass} bg-surface-accent-selected mb-6">
				<div class="flex flex-row items-center gap-2 min-w-0">
					<MessageSquareDashed class="w-4 h-4 shrink-0 text-accent" />
					<!-- Names no cause: deleted, another browser, cleared site data and a
					     damaged link are indistinguishable from here. The empty session it
					     landed on is on screen, so the notice doesn't describe it. -->
					<span class="text-emphasis font-semibold">We couldn't find that session</span>
				</div>
				<Button
					variant="subtle"
					unifiedSize="2xs"
					iconOnly
					startIcon={{ icon: X }}
					title="Dismiss"
					aria-label="Dismiss"
					onclick={() => clearSessionRecovered(sessionId)}
				/>
			</div>
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
				<div class="{bannerClass} bg-surface-tertiary">
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

	{#snippet actingOn()}
		<SessionWorkspaceBar {session} />
	{/snippet}

	<!-- The left of AIChatDisplay's controls row: the rest arrives with the chat. -->
	{#snippet placeholderToolbar()}
		<div class="flex flex-row items-center gap-x-1.5 min-w-0 flex-wrap">
			<Button unifiedSize="2xs" variant="subtle" iconOnly startIcon={{ icon: Plus }} disabled />
			{@render actingOn()}
		</div>
	{/snippet}

	{#snippet sessionHeader()}
		<EditableInput
			bind:this={summaryInput}
			value={session.summary ?? ''}
			placeholder="Untitled session"
			onSave={(v) => renameSession(session.id, v)}
			class="text-xs font-normal"
			inputClass="!text-xs !font-normal"
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
				{
					displayName: 'Move to a fork…',
					icon: GitFork,
					// Before the first message, the "Run in" picker sets the workspace instead.
					hide: !hasFirstUserMessage || isUnavailable || session.archived,
					disabled: !canMoveToFork,
					action: () => (movePickerOpen = true)
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
		{#if actingWorkspaceId}
			<!-- Opened from the menu item above; the empty trigger only anchors it. Picking a fork
			     moves the session there (see startMove). -->
			<WorkspaceFamilyPicker
				bind:open={movePickerOpen}
				selectedId={actingWorkspaceId}
				forksOf={actingWorkspaceId}
				onPick={(id) => startMove({ kind: 'existing', id })}
				onCreateFork={(fork) => startMove({ kind: 'new', fork })}
				createForkCaption="Created now, and the session moves into it."
				confirmForkLabel="Move here"
			>
				{#snippet trigger()}
					<span class="block w-0 h-5"></span>
				{/snippet}
			</WorkspaceFamilyPicker>
		{/if}

		<SessionShareButton
			sessionId={session.id}
			name={session.summary ?? ''}
			workspaceId={session.workspace_id}
			manager={runtime.manager}
		/>
	{/snippet}

	{#if headerInPage}
		<!-- The chat's own header row is the page header on this route: the session's name is its
		     breadcrumb, and the menu is its action. The workspace it acts on is named by the
		     breadcrumb's own workspace part, the way every other route names it — a session in a
		     fork reads like anything else in that fork. -->
		<PageHeaderContent
			section={{
				label: session.summary ?? 'Untitled session',
				content: sessionHeader,
				noNavPicker: true
			}}
			{actingWorkspaceId}
		/>
	{/if}

	<!-- The wrapper contributes only the chat column; edited items are shown in the
	     page's preview tabs (PreviewTabHost) beside it, not a second pane here. The
	     single Pane fills 100% (no explicit `size`, so Splitpanes auto-distributes). -->
	<div class="flex-1 min-h-0 flex flex-col" use:splitterPointerCapture>
		<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
			<Pane minSize={25} class="flex flex-col min-h-0 pb-2">
				<div class="flex-1 min-h-0 w-full flex flex-col">
					{#if showChat && aiChatComponent.current}
						<aiChatComponent.current
							bind:this={aiChat}
							hideHeader
							hideModeSelector
							wideLayout
							initialInstructions={draftText}
							onDraftChange={(text) => setSessionDraftPrompt(sessionId, text)}
							initialAttachments={armedAtInit ? undefined : getSessionDraftAttachments(sessionId)}
							onAttachmentsChange={(a) => setSessionDraftAttachments(sessionId, a)}
							placeholder={composerPlaceholder}
							forceDisabled={isUnavailable || !!session.archived}
							forceDisabledMessage={isUnavailable
								? 'This session is linked to a workspace that no longer exists. Move it or discard it from the banner above to keep working.'
								: session.archived
									? 'This session is archived. Unarchive it from the banner above to keep working.'
									: ''}
							starter={opensSession
								? { where: 'session', suggestions: starterPrompts.suggestions }
								: undefined}
							viewTransitionNames={headerInPage && (opensSession || arrivingFromHomeSend())}
							emptyHint={sessionEmptyHint}
							{inputPreface}
							actingOn={hasFirstUserMessage ? undefined : actingOn}
						/>
					{:else}
						<!-- AIChatDisplay's layout, so the composer stays put when the chat replaces it:
						     its transcript area, its column, and its footer row's height; for a session
						     that has never sent, the greeting above the centred composer. -->
						<div
							class="flex flex-col h-full {opensSession ? 'justify-center' : ''}"
							{@attach () => () =>
								runtime.manager.setComposerHasDraft(PLACEHOLDER_DRAFT_KEY, false)}
						>
							{#if opensSession}
								<div class="flex-1 min-h-0 flex flex-col justify-end">
									<div class="w-full max-w-3xl mx-auto px-7">
										<SessionGreeting />
									</div>
								</div>
							{:else if runtime.manager.displayMessages.length > 0}
								<div class="flex-1 min-h-0"></div>
							{/if}
							<div class="w-full max-w-3xl mx-auto px-7 pb-2">
								{@render inputPreface()}
								<ComposerPlaceholder
									bind:value={() => draftText, setPlaceholderText}
									toolbar={opensSession ? placeholderToolbar : undefined}
									placeholder={opensSession
										? STARTER_PLACEHOLDER
										: runtime.manager.displayMessages.length > 0
											? 'Ask followup'
											: composerPlaceholder}
									disabled={isUnavailable || !!session.archived}
									autofocus={sessionState.currentSessionId === sessionId && !covered}
									onSelectionChange={(selection) => (handoff = selection ?? null)}
									onCompositionChange={(composing) => (placeholderComposing = composing)}
								/>
								{#if !opensSession}
									<div class="mt-1 h-5"></div>
								{:else if !isUnavailable && !session.archived && starterPrompts.suggestions.length > 0}
									<div class="pt-2">
										<ComposerSuggestions
											suggestions={starterPrompts.suggestions}
											wrap
											onPick={setPlaceholderText}
										/>
									</div>
								{/if}
							</div>
							{#if opensSession}
								<div class="flex-1 min-h-0"></div>
							{/if}
						</div>
					{/if}
				</div>
			</Pane>
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
					>{session?.summary ?? 'Untitled session'}</span
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

	{#if moveRequest}
		<SessionMoveModal
			open={moveRequest.open}
			parentName={moveRequest.parentName}
			targetName={moveRequest.targetName}
			newFork={moveRequest.target.kind === 'new'}
			items={moveRequest.items}
			conflicts={moveRequest.conflicts}
			error={moveRequest.error}
			loading={moving}
			bind:removeFromParent={moveRemoveFromParent}
			onConfirmed={() => void confirmMove()}
			onCanceled={() => (moveRequest = undefined)}
		/>
	{/if}

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
					>{session?.summary ?? 'Untitled session'}</span
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
