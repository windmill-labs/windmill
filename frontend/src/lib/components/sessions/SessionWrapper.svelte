<script module lang="ts">
	import { preloadComponent } from '$lib/utils/preloadComponent.svelte'

	// AIChat's code is most of this page's: loaded beside it rather than as part of it, so the
	// page and its ComposerPlaceholder show without waiting on it.
	const aiChatComponent = preloadComponent(
		() => import('$lib/components/copilot/chat/AIChat.svelte')
	)
</script>

<script lang="ts">
	import { setContext, untrack } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import type AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import ComposerPlaceholder from '$lib/components/copilot/chat/ComposerPlaceholder.svelte'
	import {
		globalComposerSuggestion,
		type ComposerSelection
	} from '$lib/components/copilot/chat/composerBox'
	import { useSessionPermissions } from './sessionPermissions.svelte'
	import { useWorkModeLock } from './workModeLock.svelte'
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
	import { localRunEnded, localRunStarted } from './sessionSync.svelte'
	import { sendUserToast } from '$lib/toast'
	import { truncate } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { Archive, ArchiveRestore, EllipsisVertical, GitFork, Pencil, Trash2 } from 'lucide-svelte'
	import SessionChangesBar from './SessionChangesBar.svelte'
	import { sessionHandEdits } from './sessionHandEdits.svelte'
	import SessionShareButton from './SessionShareButton.svelte'
	import {
		deleteSessionsForWorkspace,
		getEffectiveWorkspaceId,
		moveSessionToNewFork,
		moveSessionToWorkspace,
		getSessionDraftPrompt,
		setSessionDraftPrompt,
		getSessionDraftAttachments,
		setSessionDraftAttachments,
		peekSessionAutoSend,
		getSessionAutoSendDomElement,
		takeSessionAutoSend,
		reconcileAfterWorkspaceChange,
		renameSession,
		sessionState,
		setSessionArchived,
		syncWorkspaceTo,
		withOpenSessionTeardown,
		type Session,
		type SessionPreviewTab
	} from './sessionState.svelte'
	import { leaveForHome } from './sessionSwitch.svelte'
	import { workspaceRootId } from './sessionScope.svelte'
	import { fixupUrlAfterWorkspaceSwitch } from '$lib/components/sidebar/workspaceSwitchUrl'
	import { getOrCreateRuntime, removeSession } from './sessionRuntime.svelte'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'
	import { AIWorkMode } from '$lib/components/copilot/chat/workMode'
	import { resolvePreviewTab } from './previewRouter'
	import { whereIs } from './sessionPreviewTabs.svelte'

	let {
		sessionId,
		headerInPage = false,
		covered = false,
		previewTabName
	}: {
		sessionId: string
		/** True for the session the page is showing: its header rides the page header bar. */
		headerInPage?: boolean
		/** True while the page holds this wrapper inert under an overlay, where nothing takes focus. */
		covered?: boolean
		/** The name the preview's tab strip shows for a tab, without its side marker. */
		previewTabName: (tab: SessionPreviewTab) => string
	} = $props()

	// Parent keys by sessionId; this wrapper only mounts when the session exists.
	// Captured at script-init so we can synchronously bind context.
	const initialSession = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = initialSession ? getOrCreateRuntime(initialSession) : undefined

	if (runtime) {
		setContext<AIChatManager>('aiChatManager', runtime.manager)
	}

	// Reactive session reference (mutations to summary/target propagate via the $state proxy).
	// Falls back to the mounted session once it is deleted: the layout's header reads this
	// through PageHeaderContent, outside the page's tree, once more before the wrapper unmounts.
	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId) ?? initialSession)

	const permissions = useSessionPermissions(
		() => runtime?.manager.operatingWorkspace,
		() => runtime?.manager
	)
	const workModeLock = useWorkModeLock(() => session)
	// Whether the first message goes out in Use. Not `permissions.readOnly`: before that send a
	// staged fork's session reads the workspace it forks, while the message builds in the fork.
	const heldInUse = $derived(!!workModeLock.current || runtime?.manager.workMode === AIWorkMode.USE)
	// The item an unsent session opened on ("Open in AI session", or a tab opened before the
	// first send): the chat reads the displayed tab with each message, so the empty chat asks
	// about that item. What is on screen only: a collapsed panel hides it. Named as its tab is,
	// so a deployed item in View mode gets its summary from the listing. The first send commits
	// the workspace, so an unsent session is one without it.
	const previewedItem = $derived.by(() => {
		if (session?.workspace_id) return undefined
		const tab = runtime?.previewTabs.displayedTab
		if (!tab) return undefined
		const slot = resolvePreviewTab(whereIs(tab))
		if (slot.kind !== 'editor' && slot.kind !== 'viewer') return undefined
		const kind = slot.kind === 'editor' ? slot.editorKind : slot.viewerKind
		return {
			kind: kind === 'raw_app' ? 'app' : kind,
			name: previewTabName(tab)
		}
	})

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
			const domElement = getSessionAutoSendDomElement(sessionId)
			// takeSessionAutoSend clears the intent whether or not it is still
			// fresh, so this cannot re-enter on the next dependency change.
			if (!takeSessionAutoSend(sessionId) || !prompt) return
			// sendOrQueue, not sendRequest: a reused session may already be mid-turn.
			// The runtime's beforeSend awaits loadCopilot for the committed
			// workspace, so this cannot race the model config on a cold session.
			runtime.manager.sendOrQueue(prompt, domElement ? [domElement] : undefined)
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

	// After archiving or deleting the open session, Home: the session the user was on
	// is no longer usable, and routing to a sibling would feel arbitrary. The URL has to
	// leave it, or the page would fall through to recovery or stay on the archived one.
	async function leaveRemovedSession() {
		await leaveForHome()
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
			await leaveRemovedSession()
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
		await leaveRemovedSession()
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
		await leaveRemovedSession()
	}

	// Focus the chat input whenever this session is the active one.
	// The textarea is disabled until copilotInfo loads (otherwise focus is
	// a silent no-op), so we wait for that too. Triggers on initial mount,
	// warm-session switch via the picker, and the moment copilot finishes
	// loading.
	let aiChat: AIChat | undefined = $state(undefined)
	$effect(() => {
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
		// Asked because the user is switching the app to the target: it switches either way.
		switching?: { landOnHome: boolean }
	}
	let movePickerOpen = $state(false)
	let moveRequest = $state<MoveRequest | undefined>(undefined)
	let moveRemoveFromParent = $state(false)
	let moving = $state(false)

	// Every chat of the session counts, not only the one on screen: a user who opened a new
	// chat inside the session still expects the earlier edits to come along. So do the
	// items the user edited by hand in its tabs, as in the Changes bar.
	function sessionMask(): Set<string> {
		const m = runtime?.manager
		if (!m) return new Set()
		return new Set([
			...m.historyManager.getAllSavedChats().flatMap((c) => c.modifiedItems ?? []),
			...(m.modifiedItems ?? []),
			...sessionHandEdits(sessionId)
		])
	}

	// The list comes from the server, so an edit still debounced would be left out of it.
	async function listMoveItems(
		parent: string,
		target: MoveTarget
	): Promise<Pick<MoveRequest, 'items' | 'conflicts'> | string> {
		const unsaved = await flushPendingSaves(parent)
		if (unsaved) return unsaved
		const mask = sessionMask()
		try {
			// Only the user's own rows move; a legacy workspace-level draft is nobody's.
			const items = (await DraftService.listDrafts({ workspace: parent }))
				.filter((r) => r.mine && !r.legacy_draft && maskHasDraftRow(mask, r))
				.map((r) => ({ kind: r.kind, path: r.path, label: r.draft_path ?? r.path }))
			let conflicts: DraftItemRef[] = []
			if (target.kind === 'existing' && !target.replace && items.length > 0) {
				const inFork = new Set(
					(await DraftService.listDrafts({ workspace: target.id }))
						.filter((r) => r.mine && !r.legacy_draft)
						.map((r) => `${r.kind}:${r.path}`)
				)
				conflicts = items
					.filter((i) => inFork.has(`${i.kind}:${i.path}`))
					.map(({ kind, path }) => ({ kind, path }))
			}
			return { items, conflicts }
		} catch (e: any) {
			return `Could not list the session's drafts: ${e?.body ?? e?.message ?? e}`
		}
	}

	function itemKeys(items: DraftItemRef[]): string {
		return items
			.map((i) => `${i.kind}:${i.path}`)
			.sort()
			.join('\n')
	}

	// A running turn cannot be moved, so the pick stays a plain switch.
	function offerMoveOnSwitch(id: string, landOnHome: boolean): boolean {
		const from = actingWorkspaceId
		if (!session?.workspace_id || !from || id === from || !canMoveToFork || moving) return false
		if (id === $workspaceStore) return false
		if (workspaceRootId(id, $userWorkspaces) !== workspaceRootId(from, $userWorkspaces)) {
			return false
		}
		void startMove({ kind: 'existing', id }, { landOnHome })
		return true
	}

	function switchAppTo(id: string, landOnHome: boolean) {
		syncWorkspaceTo(id)
		void fixupUrlAfterWorkspaceSwitch(id, { landOnHome })
	}

	function switchWithoutMoving() {
		const req = moveRequest
		moveRequest = undefined
		if (req?.switching && req.target.kind === 'existing') {
			switchAppTo(req.target.id, req.switching.landOnHome)
		}
	}

	async function startMove(target: MoveTarget, switching?: { landOnHome: boolean }) {
		const parent = actingWorkspaceId
		if (!session || !parent || !canMoveToFork || moving) return
		if (target.kind === 'existing' && target.id === parent) return
		const listed = await listMoveItems(parent, target)
		if (typeof listed === 'string') {
			sendUserToast(listed, true)
			if (switching && target.kind === 'existing') switchAppTo(target.id, switching.landOnHome)
			return
		}
		moveRemoveFromParent = false
		moveRequest = {
			open: !!switching || listed.items.length > 0,
			switching,
			parent,
			parentName: $userWorkspaces.find((w) => w.id === parent)?.name ?? parent,
			target,
			targetName:
				target.kind === 'new'
					? target.fork.name
					: ($userWorkspaces.find((w) => w.id === target.id)?.name ?? target.id),
			...listed
		}
		if (!moveRequest?.open) await confirmMove()
	}

	async function confirmMove() {
		const req = moveRequest
		if (!session || !req || moving) return
		// Another tab moved the session while this dialog was open.
		if (actingWorkspaceId !== req.parent) {
			moveRequest = undefined
			sendUserToast(
				'This session moved to another workspace meanwhile. Pick the workspace again.',
				true
			)
			return
		}
		// A turn started while the dialog was open (a job's auto-resume) owns the cross-tab hold
		// below; the move would release it while that turn still runs.
		if (!canMoveToFork) {
			moveRequest = { ...req, open: true, error: 'A chat turn is running. Retry once it ends.' }
			return
		}
		moving = true
		if (runtime) runtime.manager.moveStarted = true
		const releaseAutoResume = runtime?.manager.holdAutoResume()
		// Bracketed like a turn, so another tab on this session cannot send while its drafts
		// move, then re-reads the session (its new workspace, the model note) once it ends.
		const chatId = runtime?.manager.historyManager.getCurrentChatId()
		if (chatId) localRunStarted(session.id, chatId)
		try {
			// Closing the draft editors drops what they have not handed to their save yet, so they
			// are drained while still mounted, then closed with no await in between.
			const unsaved = await flushPendingSaves(req.parent)
			if (unsaved) {
				moveRequest = { ...req, open: true, error: unsaved }
				return
			}
			if (runtime) runtime.manager.moving = true
			// A turn that ran while the dialog was open (a job's auto-resume, another tab) may have
			// added drafts the list does not show, and the conflicts listed may have been discarded
			// since. Now that no turn can start, both are re-read.
			const listed = await listMoveItems(req.parent, req.target)
			if (typeof listed === 'string') {
				moveRequest = { ...req, open: true, error: listed }
				return
			}
			const changed = itemKeys(listed.items) !== itemKeys(req.items)
			if (changed || listed.conflicts.length > 0) {
				moveRequest = {
					...req,
					...listed,
					open: true,
					error: changed
						? "This session's drafts changed. Check the list and confirm again."
						: undefined
				}
				return
			}
			let relisted: Awaited<ReturnType<typeof listMoveItems>> | undefined
			const res = await moveSessionToFork(session.id, req.parent, req.target, {
				items: req.items.map(({ kind, path }) => ({ kind, path })),
				removeFromParent: moveRemoveFromParent,
				isBusy: () => !canMoveToFork,
				itemsChanged: async () => {
					relisted = await listMoveItems(req.parent, req.target)
					return typeof relisted === 'string' || itemKeys(relisted.items) !== itemKeys(req.items)
				}
			})
			if (!res.ok) {
				if (res.changed && relisted !== undefined) {
					moveRequest = {
						...req,
						...(typeof relisted === 'string' ? {} : relisted),
						target: res.retry ?? req.target,
						open: true,
						error: typeof relisted === 'string' ? relisted : res.error
					}
				} else if (!req.open) {
					sendUserToast(res.error, true)
					moveRequest = undefined
				} else {
					moveRequest = {
						...req,
						target: res.retry ?? req.target,
						error: res.error,
						conflicts: res.conflicts ?? []
					}
				}
				return
			}
			runtime?.resetPipelineEditors()
			moveRequest = undefined
			await runtime?.manager
				.queueModelNote(
					`This session moved from workspace "${req.parent}" to workspace "${res.forkId}": every further tool call acts on "${res.forkId}". ` +
						(res.copied > 0
							? `${res.copied} draft(s) of this session were copied there` +
								(res.removed > 0 ? ` and removed from "${req.parent}". ` : '. ')
							: '') +
						`Jobs from earlier turns ran in "${req.parent}".`
				)
				.catch((e) => console.error('Failed to persist the move note', e))
			runtime?.manager.rebuildGlobalSystemMessage()
			sendUserToast(`Session moved to ${req.targetName}`)
			if (req.switching) switchAppTo(res.forkId, req.switching.landOnHome)
		} finally {
			moving = false
			if (runtime) {
				runtime.manager.moving = false
				runtime.manager.moveStarted = false
			}
			releaseAutoResume?.()
			if (chatId)
				localRunEnded(session.id, runtime?.manager.historyManager.getCurrentChatId() ?? chatId)
		}
	}

	// Chrome shared by the banners stacked above the composer, so restyling one
	// can't leave the others behind. Each supplies its own background: two bg-*
	// utilities on one element resolve by stylesheet order, not attribute order.
	const bannerClass =
		'flex flex-row items-center justify-between gap-2 py-2 px-3 text-xs border rounded-md'

	// A placeholder that wraps grows the composer, so a long summary is cut to keep it on one line.
	const previewedItemPlaceholder = $derived.by(() => {
		if (!previewedItem) return undefined
		const item = `${previewedItem.kind} ${truncate(previewedItem.name, 24, '…')}`
		return heldInUse
			? `Ask about the ${item}, or run it…`
			: `Ask about the ${item}, or describe a change…`
	})
</script>

{#if !session || !runtime}
	<div class="p-8 text-secondary text-sm">Session not found</div>
{:else}
	<!-- A session with no message yet, or whose transcript is still loading, keeps its composer
	     at the bottom, where it stays once the transcript shows, rather than the chat's default
	     keyboard-shortcut hint, which means nothing here. -->
	{#snippet sessionEmptyHint()}<div class="flex-1 min-h-0"></div>{/snippet}

	{#snippet inputPreface()}
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

	<!-- Read by the page header bar, outside this branch, so `session` can be gone here: a delete
	     removes the session while the bar still shows it, on the way to the next page. The
	     section's label below reads it the same way. -->
	{#snippet sessionHeader()}
		{#if session}{@render sessionHeaderControls(session)}{/if}
	{/snippet}

	{#snippet sessionHeaderControls(session: Session)}
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
					displayName: 'Move to another workspace…',
					icon: GitFork,
					// Unsent, the session's workspace is `pending_workspace_id`, which
					// `createSession` rewrites when it reuses the blank session.
					hide: !session.workspace_id || isUnavailable || session.archived,
					disabled: !canMoveToFork || moving,
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
				moveFrom={actingWorkspaceId}
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
				label: session?.summary ?? 'Untitled session',
				content: sessionHeader,
				noNavPicker: true
			}}
			{actingWorkspaceId}
			onPickWorkspace={offerMoveOnSwitch}
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
							placeholder={previewedItemPlaceholder ?? composerPlaceholder}
							forceDisabled={isUnavailable || !!session.archived || moving}
							forceDisabledMessage={isUnavailable
								? 'This session is linked to a workspace that no longer exists. Move it or discard it from the banner above to keep working.'
								: session.archived
									? 'This session is archived. Unarchive it from the banner above to keep working.'
									: moving
										? `Moving this session to ${moveRequest?.targetName ?? 'the fork'}…`
										: ''}
							emptyHint={sessionEmptyHint}
							{inputPreface}
							workModeLock={workModeLock.current}
						/>
					{:else}
						<!-- AIChatDisplay's layout, so the composer stays put when the chat replaces it:
						     its transcript area, its column, and its footer row's height. -->
						<div
							class="flex flex-col h-full"
							{@attach () => () =>
								runtime.manager.setComposerHasDraft(PLACEHOLDER_DRAFT_KEY, false)}
						>
							<div class="flex-1 min-h-0"></div>
							<div class="w-full max-w-3xl mx-auto px-7 pb-2">
								{@render inputPreface()}
								<ComposerPlaceholder
									bind:value={() => draftText, setPlaceholderText}
									placeholder={runtime.manager.displayMessages.length > 0
										? 'Ask followup'
										: (previewedItemPlaceholder ?? composerPlaceholder)}
									disabled={isUnavailable || !!session.archived}
									autofocus={sessionState.currentSessionId === sessionId && !covered}
									onSelectionChange={(selection) => (handoff = selection ?? null)}
									onCompositionChange={(composing) => (placeholderComposing = composing)}
								/>
								<div class="mt-1 h-5"></div>
							</div>
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
			newFork={moveRequest.target.kind === 'new'
				? moveRequest.target.fork.parent_workspace_id === moveRequest.parent
				: !!moveRequest.target.replace}
			items={moveRequest.items}
			conflicts={moveRequest.conflicts}
			error={moveRequest.error}
			loading={moving}
			bind:removeFromParent={moveRemoveFromParent}
			onSwitchOnly={moveRequest.switching ? switchWithoutMoving : undefined}
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
