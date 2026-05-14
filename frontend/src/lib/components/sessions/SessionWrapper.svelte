<script lang="ts">
	import { setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { workspaceStore } from '$lib/stores'
	import { loadCopilot } from '$lib/aiStore'
	import { EllipsisVertical, PanelRightClose, PanelRightOpen, Pencil, Trash2 } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import WorkspaceItemDrillPicker from '$lib/components/WorkspaceItemDrillPicker.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import FlowEditorView from './FlowEditorView.svelte'
	import ScriptEditorView from './ScriptEditorView.svelte'
	import AppEditorView from './AppEditorView.svelte'
	import RawAppEditorView from './RawAppEditorView.svelte'
	import SessionWorkspaceBar from './SessionWorkspaceBar.svelte'
	import SessionForkBar from './SessionForkBar.svelte'
	import {
		commitSessionWorkspace,
		getEffectiveWorkspaceId,
		persistSessions,
		sessionState,
		setSessionTarget,
		type SessionTarget
	} from './sessionState.svelte'
	import { editorWarmIds, getOrCreateRuntime, removeSession } from './sessionRuntime.svelte'
	import { goto } from '$lib/navigation'
	import { slide } from 'svelte/transition'

	let { sessionId }: { sessionId: string } = $props()

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

	$effect(() => {
		if ($workspaceStore) {
			loadCopilot($workspaceStore)
		}
	})

	let summaryInput: EditableInput | undefined = $state(undefined)

	async function handleDelete() {
		if (!session) return
		const label = session.summary ?? session.name
		if (!window.confirm(`Delete session "${label}"? This cannot be undone.`)) return
		removeSession(session.id)
		const next = sessionState.sessions[0]
		if (next) await goto(`/sessions?session_name=${encodeURIComponent(next.name)}`)
		else await goto('/sessions')
	}

	// Workspace bar is shown only before the session sends its first user
	// message — after that the session's workspace is immutable.
	const hasFirstUserMessage = $derived(
		runtime?.manager.displayMessages.some((m) => m.role === 'user') ?? false
	)

	// Commit pending workspace pick (or current active workspace as
	// fallback) into `workspace_id` exactly once, when the first user
	// message lands. This is the only path that defines workspace_id.
	// When a pending fork is staged, this is also where the fork is
	// materialised via the API — no orphan forks for abandoned drafts.
	let committing = $state(false)
	$effect(() => {
		if (!session || !hasFirstUserMessage || session.workspace_id || committing) return
		committing = true
		commitSessionWorkspace(session.id, $workspaceStore ?? undefined)
			.catch((e) => console.error('Failed to commit session workspace', e))
			.finally(() => {
				committing = false
			})
	})

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
		// WorkspaceItem.kind is 'flow'|'script'|'app'; raw apps are flagged
		// via item.raw_app. The diff-API uses 'raw_app' as its kind so we
		// align SessionTarget on the same canonical string.
		const kind: SessionTarget['kind'] = item.kind === 'app' && item.raw_app ? 'raw_app' : item.kind
		applyEditorTarget({ kind, path: item.path }, item.summary)
	}

	// Editor pane visibility. Toggling this just hides/shows the pane via CSS
	// — the editor stays mounted, so re-opening doesn't pay a remount cost
	// and xy-flow / Monaco keep their viewport state.
	let editorVisible = $state(true)
</script>

{#if !session || !runtime}
	<div class="p-8 text-secondary text-sm">Session not found</div>
{:else}
	{@const hasTarget =
		session.target?.kind === 'flow' ||
		session.target?.kind === 'script' ||
		session.target?.kind === 'app' ||
		session.target?.kind === 'raw_app'}
	{@const hasEditor = mountEditor && hasTarget && editorVisible}

	{#snippet sessionEmptyHint()}
		{#if !session.target}
			<span
				class="text-2xs text-tertiary text-center px-4 my-2 inline-flex items-center justify-center gap-1 flex-wrap"
			>
				Open an editor on the top right
				<PanelRightOpen class="inline-block w-3.5 h-3.5" />
				to work on a flow or script
			</span>
		{/if}
	{/snippet}

	{#snippet inputPreface()}
		{#if !hasFirstUserMessage}
			<SessionWorkspaceBar {session} />
		{/if}
		<SessionForkBar {session} />
	{/snippet}

	<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
		<Pane size={hasEditor ? 50 : 100} minSize={25} class="flex flex-col min-h-0 pb-2">
			<header class="flex flex-row items-center gap-1 pl-4 pr-1 py-2 shrink-0">
				<EditableInput
					bind:this={summaryInput}
					value={session.summary ?? ''}
					placeholder="Untitled session"
					onSave={(v) => {
						session.summary = v
						persistSessions()
					}}
					class="text-sm font-semibold"
					inputClass="!text-sm !font-semibold"
				/>
				<DropdownV2
					fixedHeight={false}
					placement="bottom-start"
					items={[
						{
							displayName: 'Rename',
							icon: Pencil,
							action: () => summaryInput?.edit()
						},
						{
							displayName: 'Delete',
							icon: Trash2,
							type: 'delete',
							action: handleDelete
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
				{#if !session.target}
					<div class="ml-auto">
						<Popover
							placement="bottom-end"
							usePointerDownOutside
							disableFocusTrap
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
				{:else if hasTarget && mountEditor && !editorVisible}
					<div class="ml-auto">
						<Button
							variant="subtle"
							unifiedSize="xs"
							startIcon={{ icon: PanelRightOpen }}
							on:click={() => (editorVisible = true)}
						>
							Show editor
						</Button>
					</div>
				{:else if hasEditor}
					<div class="ml-auto">
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
			<div class="flex-1 min-h-0 w-full flex flex-col">
				<AIChat
					hideInputBorder
					hideHeader
					hideModeSelector
					emptyHint={sessionEmptyHint}
					{inputPreface}
				/>
			</div>
		</Pane>
		{#if hasEditor && session.target}
			<Pane size={50} minSize={30} class="flex flex-col min-h-0 p-2 pl-0">
				<div
					transition:slide={{ axis: 'x', duration: 200 }}
					class="flex flex-col flex-1 min-h-0 rounded-md border border-light overflow-hidden"
				>
					{#if session.target.kind === 'flow'}
						<FlowEditorView
							{runtime}
							path={session.target.path}
							workspaceId={effectiveWorkspaceId}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'script'}
						<ScriptEditorView
							{runtime}
							path={session.target.path}
							workspaceId={effectiveWorkspaceId}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'app'}
						<AppEditorView
							{runtime}
							path={session.target.path}
							workspaceId={effectiveWorkspaceId}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'raw_app'}
						<RawAppEditorView
							{runtime}
							path={session.target.path}
							workspaceId={effectiveWorkspaceId}
							onNavigate={pickEditorTarget}
						/>
					{/if}
				</div>
			</Pane>
		{/if}
	</Splitpanes>
{/if}

<style>
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
