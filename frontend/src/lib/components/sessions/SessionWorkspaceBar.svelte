<script lang="ts" module>
	import { get } from 'svelte/store'
	import { workspaceStore } from '$lib/stores'
	import { setSessionPendingWorkspace, stopFollowingHome } from './sessionState.svelte'
	import { switchWorkspace } from '$lib/storeUtils'
	import { workspaceAIClients } from '$lib/components/copilot/lib'
	import { fixupUrlAfterWorkspaceSwitch } from '$lib/components/sidebar/workspaceSwitchUrl'

	function followPick(id: string, switchesWorkspace: boolean) {
		if (!switchesWorkspace || get(workspaceStore) === id) return
		workspaceAIClients.init(id)
		switchWorkspace(id)
		void fixupUrlAfterWorkspaceSwitch(id)
	}

	// Before the pick, which stores the session: Home would otherwise move a draft picked off
	// Home back to the workspace it shows.
	function releaseFromHome(sessionId: string, switchesWorkspace: boolean) {
		if (!switchesWorkspace) stopFollowingHome(sessionId)
	}

	/** Point a session that has not sent yet at workspace `id`, as a pick in this bar does. */
	export function pickSessionWorkspace(sessionId: string, id: string, switchesWorkspace: boolean) {
		releaseFromHome(sessionId, switchesWorkspace)
		// Pre-send only: writes the pending pick. workspace_id stays undefined until
		// the user sends their first message. The chat targets this pending workspace
		// via the manager's workspace resolver, not the global workspaceStore.
		setSessionPendingWorkspace(sessionId, id)
		followPick(id, switchesWorkspace)
	}
</script>

<script lang="ts">
	import {
		getEffectiveWorkspaceId,
		setSessionPendingFork,
		type Session
	} from './sessionState.svelte'
	import WorkspaceFamilyPicker from './WorkspaceFamilyPicker.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'

	// `switchesWorkspace`: a pick also moves the navigation workspace. For a host whose page
	// shows the navigation workspace around the composer (Home), where a pick that left it
	// behind would read as ignored; the session page follows the session's own workspace instead.
	let { session, switchesWorkspace = false }: { session: Session; switchesWorkspace?: boolean } =
		$props()

	// Effective workspace for display: committed → pending pick → active store.
	const effectiveId = $derived(getEffectiveWorkspaceId(session) ?? $workspaceStore ?? undefined)
	const pendingFork = $derived(session.pending_fork)

	function pick(id: string) {
		pickSessionWorkspace(session.id, id, switchesWorkspace)
	}

	function stageFork(req: { parent_workspace_id: string; id: string; name: string }) {
		releaseFromHome(session.id, switchesWorkspace)
		setSessionPendingFork(session.id, req)
		followPick(req.parent_workspace_id, switchesWorkspace)
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
