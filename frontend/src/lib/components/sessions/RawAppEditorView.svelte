<script lang="ts">
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
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

	let diffDrawer: DiffDrawer | undefined = $state()

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadRawApp(workspaceId, path)
	}
</script>

{#if runtime.savedRawApp.val}
	<DiffDrawer
		bind:this={diffDrawer}
		restoreDeployed={restoreFromCurrentTarget}
		restoreDraft={restoreFromCurrentTarget}
	/>
{/if}
<SessionEditorTarget
	{runtime}
	kind="raw_app"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	effectivePath={() => runtime.rawApp.val?.path ?? path}
>
	{#snippet editor()}
		{#if runtime.rawApp.val}
			<RawAppEditor
				bind:files={runtime.rawApp.val.files}
				bind:runnables={runtime.rawApp.val.runnables}
				bind:data={runtime.rawApp.val.data}
				bind:summary={runtime.rawApp.val.summary}
				newPath={runtime.rawApp.val.path}
				{path}
				policy={runtime.rawApp.val.policy}
				bind:savedApp={runtime.savedRawApp.val}
				newApp={!runtime.savedRawApp.val}
				{diffDrawer}
				{onNavigate}
				onDeploy={(e) => {
					// Sync the preview to deployed (raw apps deploy only from this editor).
					runtime.syncPreviewWithDeployed(workspaceId, 'raw_app', e.path)
					// Deploying clears the item's pending draft — refresh the Draft Count.
					invalidateWorkspaceDrafts(workspaceId)
				}}
				onSaveDraft={() => {
					// Saving a server draft adds/updates a draft — refresh the Draft Count so
					// the session draft bar appears/updates immediately (parity with script/flow).
					invalidateWorkspaceDrafts(workspaceId)
				}}
				defaultSidebarCollapsed
				sidebarStorageKey="raw-app-sidebar-collapsed-preview"
				defaultSplitWithPreview={false}
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
