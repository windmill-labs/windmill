<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { DraftService, ScriptService, type NewScript } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { sendUserToast } from '$lib/toast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		initialTestPanelCollapsed = false,
		isActiveSession = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		initialTestPanelCollapsed?: boolean
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	// Restore actions for the diff drawer. The previous shared
	// `loadScript`-based handler was a no-op: loadScript early-returns on the
	// already-loaded path (and would re-read the local draft anyway). Instead
	// reset the live UserDraft handle to the target baseline — the inbound
	// effect then syncs the editor preview. Mirrors /scripts/edit's restore.
	async function restoreDeployed() {
		const saved = runtime.savedScript.val
		if (!saved) {
			sendUserToast('Could not restore to deployed', true)
			return
		}
		diffDrawer?.closeDrawer()
		// Drop the backend (DB) draft too, so "deployed" sticks across a reload.
		if (saved.draft) {
			try {
				await DraftService.deleteDraft({ workspace: workspaceId, kind: 'script', path: saved.path })
				saved.draft = undefined
				// Server draft gone — refresh the session draft-bar count immediately
				// instead of waiting for an AI turn-end / tab-refocus signal.
				invalidateWorkspaceDrafts(workspaceId)
			} catch (e: any) {
				sendUserToast(`Could not delete draft: ${e?.body ?? e}`, true)
				return
			}
		}
		const deployed = structuredClone($state.snapshot(saved)) as NewScript & { draft?: unknown }
		delete deployed.draft
		UserDraft.discard<NewScript>('script', path, deployed, { workspace: workspaceId })
	}

	async function restoreDraft() {
		const backendDraft = runtime.savedScript.val?.draft as NewScript | undefined
		if (!backendDraft) {
			sendUserToast('Could not restore to draft', true)
			return
		}
		diffDrawer?.closeDrawer()
		UserDraft.discard<NewScript>('script', path, structuredClone($state.snapshot(backendDraft)), {
			workspace: workspaceId
		})
	}
</script>

{#if runtime.savedScript.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />
{/if}
<SessionEditorTarget
	{runtime}
	kind="script"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	effectivePath={() => runtime.scriptStore.val?.path ?? path}
>
	{#snippet editor()}
		{#if runtime.scriptStore.val}
			<!--
				A script with no backend version yet (AI-created, never saved or deployed
				→ savedScript undefined) is a *new* script: pass an empty initialPath so
				ScriptBuilder behaves exactly like /scripts/add — Save draft is enabled and
				creates it on first save. On that save ScriptBuilder writes savedScript back
				through the bind and sets its own initialPath to the path, flipping us into
				edit mode (Save draft + Diff) without navigating away.
			-->
			<ScriptBuilder
				bind:script={runtime.scriptStore.val}
				bind:savedScript={runtime.savedScript.val}
				initialPath={runtime.savedScript.val ? path : ''}
				initialPathChosen={true}
				neverShowMeta={true}
				fullyLoaded={!runtime.slot('script').loading}
				disableHistoryChange={true}
				{diffDrawer}
				{onNavigate}
				{initialTestPanelCollapsed}
				onSaveDraft={async (e) => {
					runtime.scheduleForkComparisonRefresh()
					// Saving a draft adds/keeps a pending draft — refresh the Draft Count.
					invalidateWorkspaceDrafts(workspaceId)
					// Re-pin parent_hash to the latest version so the next Deploy's conflict
					// check (which runs before deploy, while the session stays mounted)
					// doesn't misfire.
					try {
						const latest = await ScriptService.getScriptLatestVersion({
							workspace: workspaceId,
							path: e.path
						})
						const cur = runtime.scriptStore.val
						if (latest?.script_hash && cur) cur.parent_hash = latest.script_hash
					} catch (err) {
						console.error('Failed to sync parent_hash after save draft', err)
					}
				}}
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
