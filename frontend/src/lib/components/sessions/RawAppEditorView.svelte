<script lang="ts">
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { sendUserToast } from '$lib/toast'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import { rawAppValueToDraft, type RawAppDraft } from './appDraftCodec'
	import type {
		RawAppRuntimeLogRequester,
		RawAppRunsProvider
	} from '$lib/components/raw_apps/utils'

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

	// Restore actions for the diff drawer. A `loadRawApp`-based handler is a
	// no-op: loadRawApp early-returns on the already-loaded path (and would
	// re-read the local draft anyway). Instead reset the live UserDraft cell to
	// the target baseline — useUserDraftSync's inbound effect then syncs the
	// editor preview. Mirrors ScriptEditorView.
	async function restoreDeployed() {
		const saved = runtime.savedRawApp.val
		if (!saved) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Project the deployed value into the draft shape and reset the cell to
		// it. `UserDraft.discard` fires the canonical `value: null` delete for the
		// per-user draft (so "deployed" sticks across a reload) and resets the
		// live cell in-memory; the inbound sync projects it into the preview.
		const deployed = rawAppValueToDraft(saved.value, {
			summary: saved.summary,
			policy: saved.policy,
			custom_path: saved.custom_path
		})
		UserDraft.discard<RawAppDraft>('raw_app', path, deployed, { workspace: workspaceId })
		// Per-user draft gone — refresh the session draft-bar count immediately.
		invalidateWorkspaceDrafts(workspaceId)
	}

	function registerRuntimeLogRequester(requester: RawAppRuntimeLogRequester | undefined) {
		runtime.setRuntimeLogRequester(requester)
	}

	function registerRunsProvider(provider: RawAppRunsProvider | undefined) {
		runtime.setAppRunsProvider(provider)
	}
</script>

{#if runtime.savedRawApp.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
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
				defaultSidebarCollapsed
				sidebarStorageKey="raw-app-sidebar-collapsed-preview"
				defaultSplitWithPreview={false}
				onRuntimeLogRequester={registerRuntimeLogRequester}
				onRunsProvider={registerRunsProvider}
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
