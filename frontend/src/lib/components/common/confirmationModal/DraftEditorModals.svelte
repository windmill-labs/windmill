<script lang="ts">
	/**
	 * The draft modals every editor route mounts at its trailer:
	 *  - DraftSyncConflictModal: surfaces a 409 from the autosave pipeline.
	 *  - OtherUsersDraftsModal: other users' drafts at this path, for forking.
	 *  - StaleDraftModal: prompts when the user's draft predates the latest
	 *    deploy; open-state is computed here from the route's timestamps.
	 *
	 * `enabled` (default true) gates the block — scripts suppress it on a hash.
	 * The OtherUsersDraftsModal is `{#key path}`-wrapped so a path change fully
	 * remounts it rather than recomputing `forkPath` mid-flight.
	 */
	import type { UserDraftItemKind } from '$lib/gen'
	import DraftSyncConflictModal from './DraftSyncConflictModal.svelte'
	import OtherUsersDraftsModal, { type OtherDraftUser } from './OtherUsersDraftsModal.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$app/paths'
	import { goto } from '$app/navigation'
	import StaleDraftModal from './StaleDraftModal.svelte'
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { untrack } from 'svelte'

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		otherDraftsUsers: OtherDraftUser[]
		/** No deployed row exists: hides the OtherUsersDraftsModal's View Diff
		 *  (nothing to diff the other user's draft against). */
		draftOnly?: boolean
		/** We have our own draft here — legacy "Assign to self" confirms before
		 *  overwriting it. */
		hasOwnDraft?: boolean
		onLoadFromServer: () => void | Promise<void>
		getLocalDraft: () => unknown
		/** Bindable open-flag for the OtherUsersDraftsModal (route-owned). */
		othersModalOpen: boolean
		/** ISO timestamp of the user's draft save (`undefined` = no draft).
		 *  StaleDraftModal opens when both timestamps set and draft < deployed. */
		draftSavedAt?: string | undefined
		/** ISO timestamp of the latest deploy at this path. */
		deployedAt?: string | undefined
		/** Precise staleness inputs: the deployed version the draft forked from
		 *  (`draft_base` on the get-by-path response) and the current deployed head,
		 *  both as text whatever the kind. When both are set they drive `isStale` and
		 *  the dedup key instead of the timestamps, which drift past `deployedAt` as
		 *  you keep editing. Absent (a draft never forked from a deploy) ⇒ timestamp
		 *  fallback. */
		draftBaseVersion?: string | undefined
		deployedHeadVersion?: string | undefined
		/** Who deployed the head, named in the stale prompt. */
		deployedBy?: string | undefined
		/** Discard the draft and reload deployed (same as "Reset to deployed"). */
		onLoadLatestDeploy?: () => void | Promise<void>
		/** Opens the editor's Deployed↔Current diff from the stale prompt, so the
		 *  choice between keeping and discarding is informed. Omit where the editor
		 *  has no diff drawer; the action is then not rendered. */
		onViewDiff?: () => void | Promise<void>
		/** Runs before this editor follows its draft to the item's new path: the
		 *  editor's own draft save, which materializes text the code editor still
		 *  holds. Without it, keystrokes typed since the relocating save are lost
		 *  to the navigation. */
		onBeforeRelocate?: () => void | Promise<void>
		/** Defaults to true; set to false to suppress all modals. */
		enabled?: boolean
	}

	let {
		workspace,
		itemKind,
		path,
		otherDraftsUsers,
		draftOnly = false,
		hasOwnDraft = false,
		onLoadFromServer,
		getLocalDraft,
		othersModalOpen = $bindable(),
		draftSavedAt = undefined,
		deployedAt = undefined,
		draftBaseVersion = undefined,
		deployedHeadVersion = undefined,
		deployedBy = undefined,
		onLoadLatestDeploy,
		onViewDiff,
		onBeforeRelocate,
		enabled = true
	}: Props = $props()

	// Open the StaleDraftModal at most once per (path, draft, deploy) triple,
	// so "Keep editing" doesn't loop the alert.
	let staleAlertKey = $state<string | undefined>(undefined)
	let staleModalOpen = $state(false)

	// Prefer the version comparison over the timestamp for every kind that supplies
	// one: `draftSavedAt` advances past `deployedAt` as you keep editing, hiding the
	// staleness outright.
	const useVersion = $derived(draftBaseVersion != null && deployedHeadVersion != null)
	const isStale = $derived(
		// Both paths need a draft to be out of date: the editors clear `draftSavedAt`
		// when a deploy consumes theirs, and the version pair stays armed for the draft
		// the next edit starts, which is what the prompt would otherwise offer to
		// discard seconds after a successful deploy.
		!!onLoadLatestDeploy &&
			!!draftSavedAt &&
			(useVersion
				? draftBaseVersion !== deployedHeadVersion
				: !!deployedAt && new Date(draftSavedAt).getTime() < new Date(deployedAt).getTime())
	)
	// Key on the versions (not `draftSavedAt`) in the version path, else every
	// autosave would mint a new key and re-pop the modal mid-edit.
	const currentKey = $derived(
		isStale
			? useVersion
				? `${path}|v|${draftBaseVersion}|${deployedHeadVersion}`
				: `${path}|${draftSavedAt}|${deployedAt}`
			: undefined
	)

	$effect(() => {
		const key = currentKey
		untrack(() => {
			if (key && key !== staleAlertKey) {
				staleAlertKey = key
				staleModalOpen = true
			}
		})
	})

	const EDITOR_SEGMENT: Partial<Record<UserDraftItemKind, string>> = {
		script: 'scripts/edit',
		flow: 'flows/edit',
		app: 'apps/edit',
		raw_app: 'apps_raw/edit'
	}

	// The item was moved while this editor was open: the draft row followed it
	// and the save just landed there. Follow it too — the route reloads the item
	// at its new path, and the stale prompt above then says what changed. Edits
	// typed since that save are flushed first, so leaving this path drops none.
	$effect(() => {
		if (!enabled || !workspace || !path) return
		const seg = EDITOR_SEGMENT[itemKind]
		if (!seg) return
		const query = { workspace, itemKind, path }
		// The flush below saves again and can land here a second time, and a second move
		// can land while it runs: the last destination reported is the one to follow.
		let relocating = false
		let destination: string | undefined = undefined
		return UserDraftDbSyncer.onRelocated(query, async (newPath) => {
			destination = newPath
			if (relocating) return
			relocating = true
			await onBeforeRelocate?.()
			await UserDraftDbSyncer.flush(query)
			// `flush` resolves on a failed or rejected save as well, and leaving the
			// route drops what it was carrying: stay, so the editor keeps the edits
			// and its own failure indicator.
			if (
				UserDraftDbSyncer.getState(query).failureMessage ||
				UserDraftDbSyncer.getConflict(query).conflict
			) {
				relocating = false
				return
			}
			const target = destination ?? newPath
			sendUserToast(`This item was moved to ${target}. You are now editing it there.`)
			await goto(`${base}/${seg}/${target}`)
		})
	})
</script>

{#if enabled && workspace && path}
	<DraftSyncConflictModal
		query={{ workspace, itemKind, path }}
		{onLoadFromServer}
		{getLocalDraft}
	/>
	{#if otherDraftsUsers.length > 0}
		{#key path}
			<OtherUsersDraftsModal
				{workspace}
				{itemKind}
				{path}
				{otherDraftsUsers}
				{draftOnly}
				{hasOwnDraft}
				onReload={onLoadFromServer}
				bind:isOpen={othersModalOpen}
			/>
		{/key}
	{/if}
	{#if onLoadLatestDeploy}
		<StaleDraftModal
			bind:isOpen={staleModalOpen}
			{itemKind}
			{draftSavedAt}
			{deployedAt}
			{draftBaseVersion}
			{deployedHeadVersion}
			{deployedBy}
			{onLoadLatestDeploy}
			{onViewDiff}
		/>
	{/if}
	<ConfirmationModal
		open={OtherUserDraftLoad.isOverwriteModalOpen(workspace, itemKind, path)}
		title="Overwrite your current draft?"
		confirmationText="Overwrite"
		onConfirmed={() => OtherUserDraftLoad.confirmOverwrite(workspace, itemKind, path)}
		onCanceled={() => OtherUserDraftLoad.dismissOverwriteModal(workspace, itemKind, path)}
	>
		<span class="text-sm">
			You're editing another user's draft. Saving this edit will overwrite your own draft at this
			path. Continue?
		</span>
	</ConfirmationModal>
{/if}
