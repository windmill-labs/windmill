<script lang="ts">
	/**
	 * The modals every editor route mounts at its trailer:
	 *  - DraftSyncConflictModal: surfaces a 409 from the autosave pipeline.
	 *    The route supplies `onLoadFromServer` (re-fetches the deployed-overlay
	 *    response and re-seeds editor state) and a `getLocalDraft` getter that
	 *    snapshots the cell value for the overwrite branch.
	 *  - OtherUsersDraftsModal: surfaces the list of other workspace users (or
	 *    the legacy NULL-email row) with a draft at the same path, offered for
	 *    forking. Externally controlled via `othersModalOpen`.
	 *  - StaleDraftModal: on each load, if the authed user's draft is older
	 *    than the latest deploy at the same path, prompt the user to either
	 *    discard the draft and pick up the new deploy or keep editing the
	 *    older draft. Open-state is computed here from the timestamps passed
	 *    in by the route, so each editor doesn't re-derive the comparison.
	 *
	 * Gates the whole block on `enabled` (defaults to true) — the scripts
	 * route uses this to suppress when viewing a specific hash.
	 *
	 * Wrap the OtherUsersDraftsModal in `{#key path}` so navigating between
	 * editor paths fully remounts the modal (carrying its `jsonOpen` /
	 * fork-target state with it) instead of trying to recompute its
	 * `forkPath` derivation for a new path mid-flight.
	 */
	import type { UserDraftItemKind } from '$lib/gen'
	import DraftSyncConflictModal from './DraftSyncConflictModal.svelte'
	import OtherUsersDraftsModal, { type OtherDraftUser } from './OtherUsersDraftsModal.svelte'
	import StaleDraftModal from './StaleDraftModal.svelte'
	import { untrack } from 'svelte'

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		otherDraftsUsers: OtherDraftUser[]
		onLoadFromServer: () => void | Promise<void>
		getLocalDraft: () => unknown
		/** Bindable open-flag for the OtherUsersDraftsModal. The route owns
		 *  the state (so the AutosaveIndicator popover button can flip it on)
		 *  and binds it here. */
		othersModalOpen: boolean
		/** ISO timestamp the authed user's draft was saved (`undefined` =
		 *  no draft loaded). Drives the StaleDraftModal alongside
		 *  `deployedAt` — open when both are set and draft < deployed. */
		draftSavedAt?: string | undefined
		/** ISO timestamp of the latest deploy at this path. */
		deployedAt?: string | undefined
		/** Discard the user's draft and reload deployed — same callback the
		 *  AutosaveIndicator's "Reset to deployed" button uses. Wired into
		 *  the StaleDraftModal's "Load latest deploy" button. */
		onLoadLatestDeploy?: () => void | Promise<void>
		/** Defaults to true; set to false to suppress all modals. */
		enabled?: boolean
	}

	let {
		workspace,
		itemKind,
		path,
		otherDraftsUsers,
		onLoadFromServer,
		getLocalDraft,
		othersModalOpen = $bindable(),
		draftSavedAt = undefined,
		deployedAt = undefined,
		onLoadLatestDeploy,
		enabled = true
	}: Props = $props()

	// Open the StaleDraftModal at most once per (path, draft, deploy)
	// triple. The modal also accepts a manual dismiss, and re-opening
	// it for the same triple after a "Keep editing" would loop the
	// alert. Track the triple we've already alerted on.
	let staleAlertKey = $state<string | undefined>(undefined)
	let staleModalOpen = $state(false)

	const isStale = $derived(
		!!draftSavedAt &&
			!!deployedAt &&
			!!onLoadLatestDeploy &&
			new Date(draftSavedAt).getTime() < new Date(deployedAt).getTime()
	)
	const currentKey = $derived(isStale ? `${path}|${draftSavedAt}|${deployedAt}` : undefined)

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
{/if}
