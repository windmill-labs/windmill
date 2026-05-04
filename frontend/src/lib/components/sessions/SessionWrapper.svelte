<script lang="ts">
	import { setContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import EditableInput from '$lib/components/common/EditableInput.svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { AIChatManager, AIMode } from '$lib/components/copilot/chat/AIChatManager.svelte'
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
	import { persistSessions, sessionState, type SessionTarget } from './sessionState.svelte'
	import { getOrCreateRuntime, removeSession } from './sessionRuntime.svelte'
	import { goto } from '$lib/navigation'
	import { slide } from 'svelte/transition'

	let { sessionId, mountEditor = true }: { sessionId: string; mountEditor?: boolean } = $props()

	// Parent keys by sessionId; this wrapper only mounts when the session exists.
	// Captured at script-init so we can synchronously bind context.
	const initialSession = sessionState.sessions.find((s) => s.id === sessionId)
	const runtime = initialSession ? getOrCreateRuntime(initialSession) : undefined

	if (runtime) {
		setContext<AIChatManager>('aiChatManager', runtime.manager)
	}

	// Reactive session reference (mutations to summary/target propagate via the $state proxy)
	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))

	let initialModeUpgraded = false
	$effect(() => {
		if (initialModeUpgraded || !runtime || !session) return
		const kind = session.target?.kind
		if (kind === 'flow' && runtime.manager.allowedModes.flow) {
			runtime.manager.mode = AIMode.FLOW
			initialModeUpgraded = true
		} else if (kind === 'script' && runtime.manager.allowedModes.script) {
			runtime.manager.mode = AIMode.SCRIPT
			initialModeUpgraded = true
		} else if ((kind === 'app' || kind === 'rawapp') && runtime.manager.allowedModes.app) {
			runtime.manager.mode = AIMode.APP
			initialModeUpgraded = true
		}
	})

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

	function pickEditorTarget(item: WorkspaceItem) {
		if (!session) return
		// WorkspaceItem.kind is 'flow'|'script'|'app'; raw apps are flagged via
		// item.raw_app. Our SessionTarget keeps 'rawapp' as a separate kind so
		// the wrapper routes to RawAppEditorView.
		const kind: SessionTarget['kind'] = item.kind === 'app' && item.raw_app ? 'rawapp' : item.kind
		session.target = { kind, path: item.path }
		if (!session.summary && item.summary) session.summary = item.summary
		persistSessions()
		// Picking a target also re-opens the editor pane (the user just chose
		// what to view).
		editorVisible = true
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
		session.target?.kind === 'rawapp'}
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
			<div class="flex-1 min-h-0 w-full max-w-3xl mx-auto flex flex-col">
				<div class="flex-1 min-h-0">
					<AIChat hideInputBorder hideHeader emptyHint={sessionEmptyHint} {inputPreface} />
				</div>
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
							workspaceId={session.workspace_id}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'script'}
						<ScriptEditorView
							{runtime}
							path={session.target.path}
							workspaceId={session.workspace_id}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'app'}
						<AppEditorView
							{runtime}
							path={session.target.path}
							workspaceId={session.workspace_id}
							onNavigate={pickEditorTarget}
						/>
					{:else if session.target.kind === 'rawapp'}
						<RawAppEditorView
							{runtime}
							path={session.target.path}
							workspaceId={session.workspace_id}
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
