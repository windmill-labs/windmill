<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { ScriptService, type NewScript } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import SessionItemNotFound from './SessionItemNotFound.svelte'
	import { sendUserToast } from '$lib/toast'

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
		/**
		 * Only the visible session should claim the workspace's live-editor
		 * slot — without this, a hidden warm-mounted session can overwrite the
		 * active session's UserDraft live-editor target (one slot per
		 * (workspace, kind)), so chat actions like discard / "the open editor"
		 * resolve to the wrong session.
		 */
		isActiveSession?: boolean
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadScript(workspaceId, path))
		}
	})

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
		// Drop the user's per-user draft too, so "deployed" sticks across
		// a reload. The new overlay folds the draft into the response (no
		// separate `.draft` field), so `saved.is_draft` is the signal that
		// there's actually a draft worth deleting; the syncer's
		// `value: null` POST is the canonical per-user delete.
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

	// Mark this editor as the live editor draft for the session's workspace
	// so the chat's `isLiveDraft` hint / `discard_local_draft` tool resolve
	// to this path — same registration the regular /scripts/edit page does.
	// Gated on `isActiveSession`: warm-but-hidden session editors must not
	// claim the workspace's single live-editor slot, else chat actions on the
	// visible session resolve to the hidden one's path.
	$effect(() => {
		if (!workspaceId || !path) return
		if (!isActiveSession) return
		UserDraft.setLiveEditorDraft({
			workspace: workspaceId,
			itemKind: 'script',
			storagePath: path,
			effectivePath: runtime.scriptStore.val?.path ?? path
		})
		return () =>
			UserDraft.clearLiveEditorDraft('script', { workspace: workspaceId, storagePath: path })
	})

	// Bidirectional sync between this preview and `UserDraft<NewScript>`.
	// The same path under the same workspace is shared with the session's
	// chat (read_workspace_item / write_script / edit_script) and any other
	// open editor on the same workspace.
	//
	// We hold a *live* handle (useMany) instead of reading via the static
	// `UserDraft.get`. The handle materializes UserDraft's shared reactive
	// `$state` cell for (workspace, 'script', path) — and that cell is what
	// lets the chat's writes (UserDraft.save, from write_script / edit_script)
	// reach this preview. Without a live entry those writes only touch
	// localStorage and the inbound effect below never re-fires. A reactive
	// getter is used (not `use()`) because switching open_preview to another
	// script swaps `path` without remounting this view, so the handle must
	// re-acquire.
	//
	// One-way-reactive discipline: inbound tracks ONLY the handle's `draft`
	// (and reads `script.content` via untrack); outbound tracks ONLY
	// `script.content` (and reads UserDraft via untrack). Without that
	// asymmetry, a user keystroke would re-fire the inbound effect with the
	// pre-keystroke stored value and revert the edit.
	const draftHandles = UserDraft.useMany<NewScript>(() => [
		{ itemKind: 'script', path, workspace: workspaceId }
	])
	let lastInboundContent: string | undefined = $state(undefined)

	// Store → editor. Re-runs when the handle's draft changes (chat write,
	// other session edit, …). `script.content` is read inside untrack so user
	// keystrokes don't refire this effect.
	$effect(() => {
		if (!workspaceId || !path) return
		const draft = draftHandles[0]?.draft
		if (!draft || typeof draft.content !== 'string') return
		const incoming = draft.content
		untrack(() => {
			if (runtime.loadedScriptPath !== path) return
			const script = runtime.scriptStore.val
			if (!script) return
			if (incoming === script.content) return
			lastInboundContent = incoming
			script.content = incoming
			if (draft.language) script.language = draft.language
			if (draft.summary !== undefined) script.summary = draft.summary
		})
	})

	// Editor → store. Re-runs on `script.content` mutation (user typing
	// or inbound write). UserDraft is read inside untrack so writing here
	// doesn't ping-pong the inbound effect. `UserDraft.save` persists
	// immediately and, now that the entry is live, updates the same cell the
	// inbound effect reads (the content guard there makes it a no-op).
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedScriptPath !== path) return
		const script = runtime.scriptStore.val
		if (!script) return
		const content = script.content
		if (content === lastInboundContent) return
		untrack(() => {
			const current = UserDraft.get<NewScript>('script', path, { workspace: workspaceId })
			if (current && current.content === content) return
			UserDraft.save<NewScript>(
				'script',
				path,
				{ ...(current ?? script), ...script },
				{
					workspace: workspaceId
				}
			)
		})
	})
</script>

{#if runtime.savedScript.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} {restoreDraft} />
{/if}
{#if runtime.loadingScript && !runtime.loadedScriptPath}
	<div class="p-4 text-secondary text-sm">Loading script {path}…</div>
{:else if runtime.notFoundScript && !runtime.loadedScriptPath}
	<SessionItemNotFound kind="script" {path} {onNavigate} />
{:else if runtime.scriptStore.val}
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
		fullyLoaded={!runtime.loadingScript}
		disableHistoryChange={true}
		{diffDrawer}
		{onNavigate}
		{initialTestPanelCollapsed}
		onSaveDraft={async (e) => {
			runtime.scheduleForkComparisonRefresh()
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
		}}
	/>
{/if}
