<script lang="ts">
	import { page } from '$app/state'
	import {
		userWorkspaces,
		workspaceStore,
		workspaceColor,
		globalForkModal,
		type UserWorkspace
	} from '$lib/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import WorkspaceFamilyPicker from '$lib/components/sessions/WorkspaceFamilyPicker.svelte'
	import {
		sessionState,
		getEffectiveWorkspaceId,
		setSessionPendingWorkspace,
		setSessionPendingFork,
		syncWorkspaceTo
	} from '$lib/components/sessions/sessionState.svelte'
	import { getContrastTextColor } from '$lib/utils'
	import { Building, ChevronDown, GitFork } from 'lucide-svelte'

	function findRoot(id: string | undefined, all: UserWorkspace[]): UserWorkspace | undefined {
		if (!id) return undefined
		let current = all.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = all.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	// The breadcrumb stages a pending fork only for a not-yet-committed draft
	// session (the same pre-send semantics as the old in-chat SessionWorkspaceBar).
	// Everywhere else it switches the active workspace directly.
	const onSessionsPage = $derived(page.route.id?.includes('/sessions') ?? false)
	const currentSession = $derived(
		onSessionsPage
			? sessionState.sessions.find((s) => s.id === sessionState.currentSessionId)
			: undefined
	)
	const draftSession = $derived(
		currentSession && !currentSession.workspace_id ? currentSession : undefined
	)

	// Effective workspace for display: draft pending pick → committed → active store.
	const effectiveId = $derived(
		draftSession
			? (getEffectiveWorkspaceId(draftSession) ?? $workspaceStore ?? undefined)
			: ($workspaceStore ?? undefined)
	)
	const root = $derived(findRoot(effectiveId, $userWorkspaces))
	const currentWs = $derived(
		effectiveId ? $userWorkspaces.find((w) => w.id === effectiveId) : undefined
	)
	const pendingFork = $derived(draftSession?.pending_fork)
	const isFork = $derived(!!pendingFork || (!!currentWs && !!root && currentWs.id !== root.id))
	const forkLabel = $derived(pendingFork?.name ?? (isFork ? currentWs?.name : 'main'))

	function switchWorkspaceDirect(id: string) {
		if ($workspaceStore === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		// Leaving an item-scoped page on a workspace change would show a
		// wrong-workspace (or missing) item — go home instead, mirroring the
		// sidebar workspace picker.
		const editPages = [
			'/scripts/edit/',
			'/flows/edit/',
			'/apps/edit/',
			'/scripts/get/',
			'/flows/get/',
			'/apps/get/'
		]
		const isOnEditPage = editPages.some((p) => page.route.id?.includes(p) ?? false)
		if (isOnEditPage) {
			void goto('/')
		} else if (page.url.searchParams.get('workspace')) {
			page.url.searchParams.set('workspace', id)
		}
	}

	function pick(id: string) {
		if (draftSession) {
			// Pre-send only: stage the pending pick. workspace_id stays undefined
			// until the user sends their first message.
			setSessionPendingWorkspace(draftSession.id, id)
			syncWorkspaceTo(id)
		} else {
			switchWorkspaceDirect(id)
		}
	}

	function stageFork(req: { parent_workspace_id: string; id: string; name: string }) {
		if (!draftSession) return
		setSessionPendingFork(draftSession.id, req)
		syncWorkspaceTo(req.parent_workspace_id)
	}

	function openForkModal() {
		globalForkModal.val = { opened: true }
	}
</script>

{#if effectiveId}
	{@const iconColor = getContrastTextColor($workspaceColor)}
	<div
		class="shrink-0 h-9 flex flex-row items-center gap-1 px-3 border-b border-light text-xs bg-surface"
	>
		<!-- One breadcrumb: family · fork. Family (root) carries the visual
		     weight; the fork/main segment is de-emphasized. The whole element is
		     the picker trigger — its popover lists forks and offers to create one.
		     Family selection itself remains the sidebar picker's job. -->
		<WorkspaceFamilyPicker
			selectedId={effectiveId}
			{pendingFork}
			onPick={pick}
			onCreateFork={draftSession ? stageFork : undefined}
			onRequestCreateFork={draftSession ? undefined : openForkModal}
			createForkCaption={draftSession ? 'Created when you send your first message.' : ''}
		>
			{#snippet trigger({ open })}
				<button
					type="button"
					class="inline-flex flex-row items-center gap-1.5 px-1.5 py-1 rounded min-w-0 hover:bg-surface-hover {open
						? 'bg-surface-hover'
						: ''}"
				>
					<Building
						class="w-3.5 h-3.5 shrink-0"
						style={iconColor ? `color: ${iconColor}` : undefined}
					/>
					<span class="truncate max-w-[200px] text-primary font-medium">
						{root?.name ?? currentWs?.name ?? effectiveId}
					</span>
					<span class="shrink-0 text-tertiary">·</span>
					{#if isFork}
						<GitFork class="w-3 h-3 shrink-0 text-tertiary" />
					{/if}
					<span class="truncate max-w-[160px] text-secondary font-normal">
						{forkLabel}
					</span>
					{#if pendingFork}
						<span class="text-2xs text-tertiary italic shrink-0">(new)</span>
					{/if}
					<ChevronDown class="w-3.5 h-3.5 shrink-0 text-tertiary" />
				</button>
			{/snippet}
		</WorkspaceFamilyPicker>
	</div>
{/if}
