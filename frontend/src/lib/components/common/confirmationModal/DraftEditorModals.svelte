<script lang="ts">
	/**
	 * The two modals every editor route mounts at its trailer:
	 *  - DraftSyncConflictModal: surfaces a 409 from the autosave pipeline.
	 *    The route supplies `onLoadFromServer` (re-fetches the deployed-overlay
	 *    response and re-seeds editor state) and a `getLocalDraft` getter that
	 *    snapshots the cell value for the overwrite branch.
	 *  - OtherUsersDraftsModal: surfaces the list of other workspace users (or
	 *    the legacy NULL-email row) with a draft at the same path, offered for
	 *    forking. Open-gated on `otherDraftsUsers.length > 0`.
	 *
	 * Gates the whole block on `enabled` (defaults to true) — the scripts
	 * route uses this to suppress when viewing a specific hash.
	 *
	 * Wrap the OtherUsersDraftsModal in `{#key path}` so navigating between
	 * editor paths fully remounts the modal (carrying its `jsonOpen` /
	 * fork-target state with it) instead of trying to recompute its
	 * `forkPath` derivation for a new path mid-flight.
	 */
	import { userStore } from '$lib/stores'
	import type { UserDraftItemKind } from '$lib/gen'
	import DraftSyncConflictModal from './DraftSyncConflictModal.svelte'
	import OtherUsersDraftsModal, { type OtherDraftUser } from './OtherUsersDraftsModal.svelte'

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		otherDraftsUsers: OtherDraftUser[]
		editPathFor: (forkedPath: string) => string
		onLoadFromServer: () => void | Promise<void>
		getLocalDraft: () => unknown
		/** Defaults to true; set to false to suppress both modals. */
		enabled?: boolean
	}

	let {
		workspace,
		itemKind,
		path,
		otherDraftsUsers,
		editPathFor,
		onLoadFromServer,
		getLocalDraft,
		enabled = true
	}: Props = $props()
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
				currentUserUsername={$userStore?.username}
				{otherDraftsUsers}
				{editPathFor}
			/>
		{/key}
	{/if}
{/if}
