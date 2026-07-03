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
		/** Precise staleness inputs (flows/apps): the deployed version the draft was
		 *  forked from, and the current deployed head. When both are set they drive
		 *  `isStale` and the dedup key instead of the timestamps — exact, and stable
		 *  across autosaves (the timestamp drifts past `deployedAt` as you keep
		 *  editing). Absent (pre-feature drafts, scripts) ⇒ timestamp fallback. */
		draftBaseVersion?: number | undefined
		deployedHeadVersion?: number | undefined
		/** Discard the draft and reload deployed (same as "Reset to deployed"). */
		onLoadLatestDeploy?: () => void | Promise<void>
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
		onLoadLatestDeploy,
		enabled = true
	}: Props = $props()

	// Open the StaleDraftModal at most once per (path, draft, deploy) triple,
	// so "Keep editing" doesn't loop the alert.
	let staleAlertKey = $state<string | undefined>(undefined)
	let staleModalOpen = $state(false)

	// Prefer the exact version comparison (flows/apps) over the timestamp: the
	// draft's pinned fork base never drifts, whereas `draftSavedAt` advances past
	// `deployedAt` once you keep editing a stale draft, hiding the staleness.
	const useVersion = $derived(draftBaseVersion != null && deployedHeadVersion != null)
	const isStale = $derived(
		!!onLoadLatestDeploy &&
			(useVersion
				? draftBaseVersion !== deployedHeadVersion
				: !!draftSavedAt &&
					!!deployedAt &&
					new Date(draftSavedAt).getTime() < new Date(deployedAt).getTime())
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
			{draftSavedAt}
			{deployedAt}
			{onLoadLatestDeploy}
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
