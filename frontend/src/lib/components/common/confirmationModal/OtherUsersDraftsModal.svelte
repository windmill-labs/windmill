<script lang="ts">
	/**
	 * On-demand modal (from the AutosaveIndicator or DraftBadge popover) listing
	 * other users' drafts at this path. The owner list rides the overlay/list
	 * payload; individual drafts are fetched lazily for View Diff / Load.
	 * Parent-controlled via `isOpen` — never auto-opens.
	 */
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Users, Pencil, GitCompareArrows, Wrench } from 'lucide-svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import MigrateLegacyDraftModal from './MigrateLegacyDraftModal.svelte'
	import { fetchDeployedValueForDiff } from '$lib/components/otherUserDraftDiff'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { displayDate } from '$lib/utils'
	import { userStore } from '$lib/stores'

	export type OtherDraftUser = { username?: string | null; draft_saved_at?: string }

	type Props = {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		/** Owners from the overlay response (`username`, or `null` for the
		 *  legacy row). The authed user is filtered out server-side. */
		otherDraftsUsers: OtherDraftUser[]
		/** No deployed row exists (never deployed): hides View Diff, since there's
		 *  no deployed baseline to diff against. */
		draftOnly?: boolean
		/** We have our own draft at this path — "Migrate → Assign to self" of the
		 *  legacy row would overwrite it, so it confirms first. */
		hasOwnDraft?: boolean
		/** Reload the editor in place so it picks up the staged "Load" — we're
		 *  already on this item's edit route. */
		onReload?: () => void | Promise<void>
		/** Controlled visibility — bind from the parent. */
		isOpen: boolean
	}

	let {
		workspace,
		itemKind,
		path,
		otherDraftsUsers,
		draftOnly = false,
		hasOwnDraft = false,
		onReload,
		isOpen = $bindable()
	}: Props = $props()
	let busyFor = $state<string | null>(null)
	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let migrateOpen = $state(false)

	// Legacy (no-owner) drafts can only be resolved by workspace admins / superadmins.
	const canMigrateLegacy = $derived(!!$userStore?.is_admin || !!$userStore?.is_super_admin)

	function ownerLabel(owner: OtherDraftUser): string {
		return owner.username ?? 'Legacy draft'
	}

	function ownerKey(owner: OtherDraftUser): string {
		return owner.username ?? '__legacy__'
	}

	async function fetchDraft(owner: OtherDraftUser): Promise<unknown> {
		return (
			await DraftService.getDraftForUser({
				workspace,
				kind: itemKind,
				path,
				username: owner.username ?? undefined
			})
		).value
	}

	async function viewDiff(owner: OtherDraftUser) {
		busyFor = ownerKey(owner)
		try {
			const [draftValue, deployed] = await Promise.all([
				fetchDraft(owner),
				fetchDeployedValueForDiff(workspace, itemKind, path)
			])
			// Close this modal first — the DiffDrawer (z-index ~1100) renders below
			// Modal2 (z-1110), so leaving it open would hide the drawer behind it.
			isOpen = false
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: `${ownerLabel(owner)}'s draft vs deployed`,
				original: deployed,
				current: draftValue as any
			})
		} catch (e) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}

	async function load(owner: OtherDraftUser) {
		busyFor = ownerKey(owner)
		try {
			const value = await fetchDraft(owner)
			isOpen = false
			// Already on this item's edit route — stage + reload in place. If we
			// have our own draft, the loader enters overlay mode (no save until
			// the user confirms overwriting it).
			OtherUserDraftLoad.stage(workspace, itemKind, value, path, ownerLabel(owner), {
				navigate: false
			})
			await onReload?.()
		} catch (e) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}
</script>

<Modal2
	bind:isOpen
	title="Other users are currently working on {path}"
	fixedWidth="sm"
	fixedHeight="sm"
>
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start">
			<Users size={20} class="text-blue-500 shrink-0 mt-0.5" />
			<p class="text-sm text-secondary">
				Their drafts are independent of yours. For advanced collaboration, consider using <a
					target="_blank"
					href="https://www.windmill.dev/docs/advanced/workspace_forks">workspace forks (EE)</a
				>
			</p>
		</div>

		<ul class="divide-y border-t border-b flex-1 overflow-y-auto">
			{#each otherDraftsUsers as owner (ownerKey(owner))}
				<li class="flex items-center gap-3 py-2">
					<div class="flex-1 min-w-0 flex flex-col">
						<div class="flex items-center gap-2">
							<span
								class="text-sm font-medium text-primary truncate"
								class:italic={!owner.username}
							>
								{ownerLabel(owner)}
							</span>
							{#if !owner.username}
								<Tooltip>
									Pre-migration workspace-scoped draft (no owner). Saved before drafts became
									per-user — kept around so you can recover the content, but no current user owns
									it.
								</Tooltip>
							{/if}
						</div>
						{#if owner.draft_saved_at}
							<span class="text-2xs text-hint truncate">
								Last updated: {displayDate(owner.draft_saved_at)}
							</span>
						{/if}
					</div>
					{#if !draftOnly}
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: GitCompareArrows }}
							disabled={busyFor !== null && busyFor !== ownerKey(owner)}
							loading={busyFor === ownerKey(owner)}
							on:click={() => viewDiff(owner)}
						>
							View Diff
						</Button>
					{/if}
					<Button
						variant="default"
						size="xs"
						startIcon={{ icon: Pencil }}
						disabled={busyFor !== null && busyFor !== ownerKey(owner)}
						loading={busyFor === ownerKey(owner)}
						on:click={() => load(owner)}
					>
						Load
					</Button>
					{#if !owner.username && canMigrateLegacy}
						<Button
							variant="default"
							size="xs"
							startIcon={{ icon: Wrench }}
							on:click={() => (migrateOpen = true)}
						>
							Migrate
						</Button>
					{/if}
				</li>
			{/each}
		</ul>

		<div class="flex justify-end">
			<Button variant="default" size="sm" on:click={() => (isOpen = false)}>Close</Button>
		</div>
	</div>
</Modal2>

<DiffDrawer bind:this={diffDrawer} isFlow={itemKind === 'flow'} />

<MigrateLegacyDraftModal
	bind:isOpen={migrateOpen}
	{workspace}
	{itemKind}
	{path}
	ownDraftExists={hasOwnDraft}
	onMigrated={async () => {
		isOpen = false
		await onReload?.()
	}}
/>
