<script lang="ts">
	import FlowBuilder from '$lib/components/FlowBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { sendUserToast } from '$lib/toast'
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

	// In a session pane, "restore" just reloads from the current state — the
	// session target stays put. The Diff drawer's primary use here is viewing
	// the diff; restore is best-effort.
	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadFlow(workspaceId, path)
	}
</script>

{#if runtime.savedFlow.val}
	<DiffDrawer
		bind:this={diffDrawer}
		restoreDeployed={restoreFromCurrentTarget}
		restoreDraft={restoreFromCurrentTarget}
		isFlow
	/>
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
		     initialPath: a brand-new flow is stored under a `draft_<uuid>` path with
		     its intended name in `draft_path`; seed the builder from `draft_path`
		     (as the full-page editor does) so the Path widget and deploy use the
		     friendly name rather than creating a flow literally named draft_<uuid>. -->
		<FlowBuilder
			flowStore={runtime.flowStore}
			flowStateStore={runtime.flowStateStore}
			initialPath={(runtime.savedFlow.val as any)?.draft_path ?? path}
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
