<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import type { NewScript } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { sendUserToast } from '$lib/toast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		fullscreen = false,
		isActiveSession = true,
		active = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/** Preview panel is in full screen: collapse the test pane in the narrow
		 * side-by-side layout, reopen it when there's room in full screen. */
		fullscreen?: boolean
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
		/** Whether this is the visible preview tab (forwarded as isActiveTab). */
		active?: boolean
	} = $props()

	// This tab's own script cell; each open script editor binds its own store.
	const cell = $derived(runtime.scriptCell(path))
	let diffDrawer: DiffDrawer | undefined = $state()

	// Restore actions for the diff drawer. The previous shared
	// `loadScript`-based handler was a no-op: loadScript early-returns on the
	// already-loaded path (and would re-read the local draft anyway). Instead
	// reset the live UserDraft handle to the target baseline — the inbound
	// effect then syncs the editor preview. Mirrors /scripts/edit's restore.
	async function restoreDeployed() {
		const saved = cell.saved.val
		if (!saved) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Drop the user's per-user draft too, so "deployed" sticks across
		// a reload. The overlay sets `is_draft: true` when a draft exists
		// for the authed user; the syncer's `value: null` POST is the
		// canonical per-user delete.
		//
		// Fire-and-forget: every read here (`saved`, the snapshot we build
		// below, the UserDraft.discard write) is purely in-memory, so we
		// don't need the DELETE to have landed to finish the restore. We
		// flip `is_draft` optimistically so the UI matches the new intent
		// immediately. A failed DELETE only matters across a hard reload
		// before it lands — log and move on.
		if (saved.is_draft) {
			saved.is_draft = false
			UserDraftDbSyncer.save({
				workspace: workspaceId,
				itemKind: 'script',
				path: saved.path,
				value: null
			}).catch((e) => console.error('restoreDeployed: draft delete failed', e))
			// Per-user draft gone — refresh the session draft-bar count immediately
			// instead of waiting for an AI turn-end / tab-refocus signal.
			invalidateWorkspaceDrafts(workspaceId)
		}
		const deployed = structuredClone($state.snapshot(saved)) as NewScript & { draft?: unknown }
		delete deployed.draft
		UserDraft.discard<NewScript>('script', path, deployed, { workspace: workspaceId })
	}
</script>

{#if cell.saved.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
{/if}
<SessionEditorTarget
	{runtime}
	kind="script"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	isActiveTab={active}
	effectivePath={() => cell.store.val?.path ?? path}
>
	{#snippet editor()}
		{#if cell.store.val}
			<!--
				A script with no backend version yet (AI-created, never saved or deployed
				→ savedScript undefined) is a *new* script: pass an empty initialPath so
				ScriptBuilder behaves exactly like /scripts/add — Save draft is enabled and
				creates it on first save. On that save ScriptBuilder writes savedScript back
				through the bind and sets its own initialPath to the path, flipping us into
				edit mode (Save draft + Diff) without navigating away.
			-->
			<ScriptBuilder
				bind:script={cell.store.val}
				bind:savedScript={cell.saved.val}
				initialPath={cell.saved.val ? path : ''}
				autosaveWorkspace={workspaceId}
				autosavePath={path}
				initialPathChosen={true}
				neverShowMeta={true}
				fullyLoaded={!cell.slot.loading}
				disableHistoryChange={true}
				condensedHeader={true}
				{diffDrawer}
				{onNavigate}
				testPanelCollapsed={!fullscreen}
				onDeploy={(e) => {
					// Fires on every deploy (primary, "Deploy & Stay here", and lib — we
					// ignore e.stay since the session always stays). Toast, then sync the
					// preview to the deployed version.
					sendUserToast('Deployed')
					runtime.syncPreviewWithDeployed(workspaceId, 'script', e.path)
					// Deploying clears the item's pending draft — refresh the workspace
					// Draft Count so the session bar / compare page drop it immediately.
					invalidateWorkspaceDrafts(workspaceId)
				}}
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
