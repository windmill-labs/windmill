<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		setSessionPendingWorkspace,
		stopFollowingHome,
		type Session
	} from './sessionState.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import { fixupUrlAfterWorkspaceSwitch } from '$lib/components/sidebar/workspaceSwitchUrl'

	// `switchesWorkspace`: a pick also moves the navigation workspace. For a host whose page
	// shows the navigation workspace around the composer (Home), where a pick that left it
	// behind would read as ignored; the session page follows the session's own workspace instead.
	let { session, switchesWorkspace = false }: { session: Session; switchesWorkspace?: boolean } =
		$props()

	function followPick(id: string) {
		if (!switchesWorkspace || $workspaceStore === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		void fixupUrlAfterWorkspaceSwitch(id)
	}

	// Effective workspace for display: committed → pending pick → active store.
	const effectiveId = $derived(getEffectiveWorkspaceId(session) ?? $workspaceStore ?? undefined)
	const pendingFork = $derived(session.pending_fork)

	// Before the pick, which stores the session: Home would otherwise move a draft picked
	// here back to the workspace it shows.
	function releaseFromHome() {
		if (!switchesWorkspace) stopFollowingHome(session.id)
	}

	function pick(id: string) {
		releaseFromHome()
		// Pre-send only: writes the pending pick. workspace_id stays undefined until
		// the user sends their first message. The chat targets this pending workspace
		// via the manager's workspace resolver, not the global workspaceStore.
		setSessionPendingWorkspace(session.id, id)
		followPick(id)
	}

	function stageFork(req: { parent_workspace_id: string; id: string; name: string }) {
		releaseFromHome()
		setSessionPendingFork(session.id, req)
		followPick(req.parent_workspace_id)
	}
</script>

<!-- A control of the composer's toolbar; on a narrow window it gives up its label first. -->
<div class="flex flex-row items-center gap-1 min-w-0 text-2xs text-secondary">
	<span class="shrink-0 hidden lg:inline">Acting on</span>
	<WorkspaceFamilyPicker
		selectedId={effectiveId}
		{pendingFork}
		onPick={pick}
		onCreateFork={stageFork}
		createForkCaption="Created when you send your first message."
	>
		{#snippet trigger()}
			<WorkspaceScopeTrigger workspaceId={effectiveId} {pendingFork} class="max-w-[16rem]" />
		{/snippet}
	</WorkspaceFamilyPicker>
</div>
