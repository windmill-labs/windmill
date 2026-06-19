<script lang="ts">
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { Flow } from '$lib/gen'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { sendUserToast } from '$lib/toast'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		isActiveSession = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
	} = $props()

	let selectedId = $state('settings-metadata')
	let diffDrawer: DiffDrawer | undefined = $state()

	// Restore actions for the diff drawer. A `loadFlow`-based handler is a no-op:
	// loadFlow early-returns on the already-loaded path (and would re-read the
	// local draft anyway). Instead reset the live UserDraft cell to the target
	// baseline — useUserDraftSync's inbound effect then syncs the editor preview.
	// Mirrors ScriptEditorView.
	async function restoreDeployed() {
		const saved = runtime.savedFlow.val
		if (!saved) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Drop the user's per-user draft too, so "deployed" sticks across a
		// reload. The syncer's `value: null` POST is the canonical per-user
		// delete; fire-and-forget since the in-memory reset below is what the
		// preview reflects immediately.
		if (saved.is_draft) {
			saved.is_draft = false
			UserDraftDbSyncer.save({
				workspace: workspaceId,
				itemKind: 'flow',
				path: saved.path,
				value: null
			}).catch((e) => console.error('restoreDeployed: draft delete failed', e))
			invalidateWorkspaceDrafts(workspaceId)
		}
		const deployed = structuredClone($state.snapshot(saved)) as Flow & { draft?: unknown }
		delete deployed.draft
		UserDraft.discard<Flow>('flow', path, deployed, { workspace: workspaceId })
	}
</script>

{#if runtime.savedFlow.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} isFlow />
{/if}
<SessionEditorTarget
	{runtime}
	kind="flow"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	effectivePath={() => runtime.flowStore.val?.path ?? path}
>
	{#snippet editor()}
		<!-- customUi hides the in-editor "Flow AI Chat" button: the session already
		     has its own AI chat in the left pane, so the toggle is redundant here.
		     newFlow: a draft-only flow has a synthesized `savedFlow` (no_deployed=true)
		     but no deployed row, so it must deploy via createFlow — treating it as
		     !newFlow would updateFlow the draft_<uuid> path and 404 "Flow not found".
		     initialPath: the Path widget's rename baseline. For a deployed flow
		     it's the URL/storage path (deployed/original); the draft's intended
		     name rides in the flow value's own `path`, which FlowBuilder seeds into
		     the Path widget — so the topbar shows the friendly name while
		     `initialPath` stays the original. A never-deployed flow (parked at a
		     `draft_<uuid>` key) has no original, so seed the baseline from its own
		     friendly path instead of flagging the storage key as a rename. -->
		<FlowBuilder
			flowStore={runtime.flowStore}
			flowStateStore={runtime.flowStateStore}
			initialPath={runtime.savedFlow.val?.no_deployed
				? (runtime.flowStore.val?.path ?? path)
				: path}
			autosaveWorkspace={workspaceId}
			autosavePath={path}
			newFlow={!runtime.savedFlow.val || runtime.savedFlow.val.no_deployed === true}
			{selectedId}
			loading={false}
			bind:savedFlow={runtime.savedFlow.val}
			{diffDrawer}
			{onNavigate}
			customUi={{ topBar: { aiBuilder: false } }}
			onDeploy={() => {
				// FlowBuilder has no deploy toast and the session stays put, so toast
				// here, then sync the preview to deployed (pulls the new locks + version_id).
				sendUserToast('Deployed')
				runtime.syncPreviewWithDeployed(workspaceId, 'flow', path)
				// Deploying clears the item's pending draft — refresh the Draft Count.
				invalidateWorkspaceDrafts(workspaceId)
			}}
		/>
	{/snippet}
</SessionEditorTarget>
