<script lang="ts">
	/**
	 * Home-page draft badge with per-user initial circles. Hover popover lists
	 * each draft owner; when full context (workspace + itemKind + path) is
	 * passed, OTHER users' rows get inline "View Diff" / "Load" (loading
	 * yourself is meaningless, so own rows don't). draft_only → "Draft only"
	 * (no deployed row), else "Draft". Renders nothing when there's no draft.
	 */
	import Popover from './meltComponents/Popover.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { Badge } from './common'
	import Button from './common/button/Button.svelte'
	import DiffDrawer from './DiffDrawer.svelte'
	import MigrateLegacyDraftModal from './common/confirmationModal/MigrateLegacyDraftModal.svelte'
	import { GitCompareArrows, Pencil, Wrench } from 'lucide-svelte'
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { goto } from '$lib/navigation'
	import { OtherUserDraftLoad, editRouteFor } from '$lib/components/otherUserDraftLoad.svelte'
	import { fetchDeployedValueForDiff } from '$lib/components/otherUserDraftDiff'
	import { userStore } from '$lib/stores'

	type DraftUser = { username?: string | null }

	interface Props {
		is_draft?: boolean
		draft_only?: boolean
		draft_users?: DraftUser[]
		/** Authed user's username — pins their circle first and annotates `(you)`. */
		currentUsername?: string | null
		/** Context for the View Diff / Load actions. Missing → text-only popover. */
		workspace?: string
		itemKind?: UserDraftItemKind
		path?: string
		/** Called after an admin migrates (deletes / assigns) the legacy draft, so
		 *  the parent row can refetch and drop the now-resolved legacy entry. */
		onMigrated?: () => void
		/** Offer "Load" alongside "View Diff" on other users' rows. The deploy /
		 * review page sets this false: loading into a fresh editor is moot there. */
		allowFork?: boolean
		/** Compact variant: render only the avatar circles (no "Draft" pill text),
		 * slightly smaller — for tight spots like the diff-tree sidebar. The hover
		 * popover is unchanged. */
		iconOnly?: boolean
	}

	let {
		is_draft = false,
		draft_only = false,
		draft_users = [],
		currentUsername = undefined,
		workspace = undefined,
		itemKind = undefined,
		path = undefined,
		onMigrated = undefined,
		allowFork = true,
		iconOnly = false
	}: Props = $props()

	// Authed user lands first; everyone else keeps the backend's ordering.
	const orderedUsers = $derived.by(() => {
		if (!currentUsername) return draft_users
		const selfIdx = draft_users.findIndex((u) => u.username === currentUsername)
		if (selfIdx < 0) return draft_users
		const self = draft_users[selfIdx]
		const rest = draft_users.filter((_, i) => i !== selfIdx)
		return [self, ...rest]
	})

	/** Two-letter initials; `john.doe` → `JD`, `alice` → `AL`, no username → `?`. */
	function initials(u: DraftUser): string {
		const name = u.username
		if (!name) return '?'
		const parts = name.split(/[._\-\s]+/).filter(Boolean)
		if (parts.length >= 2) {
			return (parts[0][0] + parts[1][0]).toUpperCase()
		}
		return name.slice(0, 2).toUpperCase()
	}

	function fullLabel(u: DraftUser): string {
		return u.username ?? 'Legacy workspace draft'
	}

	// Deterministic color per username, same circle across rows.
	const PALETTE = [
		'bg-blue-500',
		'bg-emerald-500',
		'bg-amber-500',
		'bg-rose-500',
		'bg-violet-500',
		'bg-cyan-500'
	]
	function colorFor(u: DraftUser): string {
		const name = u.username ?? ''
		let hash = 0
		for (let i = 0; i < name.length; i++) hash = (hash * 31 + name.charCodeAt(i)) >>> 0
		return PALETTE[hash % PALETTE.length]
	}

	// First 3 circles when ≤3 users; first 2 + "+N" overflow when 4+. Sliced from
	// `orderedUsers` so the authed user (first) is never hidden in the overflow.
	const MAX_CIRCLES = 3
	const visibleUsers = $derived(
		orderedUsers.length <= MAX_CIRCLES ? orderedUsers : orderedUsers.slice(0, MAX_CIRCLES - 1)
	)
	const overflowCount = $derived(
		orderedUsers.length > MAX_CIRCLES ? orderedUsers.length - (MAX_CIRCLES - 1) : 0
	)

	// Show whenever any draft exists, or the authed user has one (`is_draft`).
	const showBadge = $derived(is_draft || draft_users.length > 0)

	// Inline actions need full context to fetch and fork drafts.
	const actionsEnabled = $derived(!!workspace && !!itemKind && !!path && draft_users.length > 0)

	const kindLabel = $derived(
		itemKind === 'flow' ? 'flow' : itemKind === 'app' || itemKind === 'raw_app' ? 'app' : 'script'
	)

	// "Only you can see this {kind}" — true when the authed user owns the
	// sole draft on a never-deployed item.
	const onlyOwnDraft = $derived(
		draft_only &&
			draft_users.length === 1 &&
			!!currentUsername &&
			draft_users[0]?.username === currentUsername
	)

	let busyFor = $state<string | null>(null)
	let diffDrawer: DiffDrawer | undefined = $state(undefined)
	let migrateOpen = $state(false)
	// The hover popover sits above the diff drawer / migrate modal (z-index), so
	// close it before opening either or it would cover them.
	let popoverOpen = $state(false)

	// Legacy (no-owner) drafts can only be resolved by workspace admins / superadmins.
	const canMigrateLegacy = $derived(!!$userStore?.is_admin || !!$userStore?.is_super_admin)

	function ownerKey(owner: DraftUser): string {
		return owner.username ?? '__legacy__'
	}

	async function fetchDraft(owner: DraftUser): Promise<unknown> {
		if (!workspace || !itemKind || !path) {
			throw new Error('Missing context for draft fetch')
		}
		return (
			await DraftService.getDraftForUser({
				workspace,
				kind: itemKind,
				path,
				username: owner.username ?? undefined
			})
		).value
	}

	async function viewDiff(owner: DraftUser) {
		if (!workspace || !itemKind || !path) return
		busyFor = ownerKey(owner)
		try {
			const [draftValue, deployed] = await Promise.all([
				fetchDraft(owner),
				fetchDeployedValueForDiff(workspace, itemKind, path)
			])
			// Close the popover so it doesn't render over the drawer.
			popoverOpen = false
			diffDrawer?.openDrawer()
			diffDrawer?.setDiff({
				mode: 'simple',
				title: `${fullLabel(owner)}'s draft vs deployed`,
				original: deployed,
				current: draftValue as any
			})
		} catch (e: any) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}

	async function load(owner: DraftUser) {
		if (!workspace || !itemKind || !path) return
		busyFor = ownerKey(owner)
		try {
			const value = await fetchDraft(owner)
			// Stage their value and open this item's editor. If we already have a
			// draft here, the editor enters overlay mode (no save until the user
			// confirms overwriting it).
			OtherUserDraftLoad.stage(workspace, itemKind, value, path, fullLabel(owner), {
				navigate: true
			})
		} catch (e: any) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}
</script>

{#snippet circleStack()}
	<!-- `-space-x-1` overlaps the circles; the indigo ring tint makes the overlap read intentional. -->
	<span class="flex -space-x-1">
		{#each visibleUsers as u, i (i)}
			<span
				class="inline-flex h-3 w-3 items-center justify-center rounded-full text-[8px] font-semibold text-white ring-1 ring-indigo-100 dark:ring-indigo-700/40 {colorFor(
					u
				)}"
				title={u.username === currentUsername ? `${fullLabel(u)} (you)` : fullLabel(u)}
			>
				{initials(u)}
			</span>
		{/each}
		{#if overflowCount > 0}
			<span
				class="inline-flex h-3 w-3 items-center justify-center rounded-full bg-gray-500 text-[8px] font-semibold text-white ring-1 ring-indigo-100 dark:ring-indigo-700/40"
				title="{overflowCount} more"
			>
				+{overflowCount}
			</span>
		{/if}
	</span>
{/snippet}

{#if showBadge}
	<!-- inline-flex/items-center so the trigger button hugs the badge and lines up
	     with sibling badges (a plain button is taller, dropping the pill ~2px). -->
	<Popover
		openOnHover={true}
		debounceDelay={50}
		enableFlyTransition
		class="inline-flex items-center"
		bind:isOpen={popoverOpen}
	>
		{#snippet trigger()}
			{#if iconOnly}
				<!-- Compact "has a draft" marker for tight spots like the diff-tree
				     sidebar: a small indigo pencil + the author avatar(s). -->
				<span class="inline-flex items-center gap-0.5" title={draft_only ? 'Draft only' : 'Draft'}>
					<Pencil class="h-3 w-3 shrink-0 text-indigo-500 dark:text-indigo-400" />
					{#if orderedUsers.length > 0}
						{@render circleStack()}
					{/if}
				</span>
			{:else}
				<Badge small color="indigo" class="px-1 py-0 gap-0.5">
					{#if orderedUsers.length > 0}
						{@render circleStack()}
					{/if}
					{draft_only ? 'Draft only' : 'Draft'}
				</Badge>
			{/if}
		{/snippet}
		{#snippet content()}
			<div class="flex flex-col gap-2 min-w-[16rem] text-xs p-4 pb-1">
				<p class="text-primary">
					{#if draft_users.length > 0}
						{draft_only ? 'Never deployed — only a draft exists.' : 'Deployed with drafts pending.'}
					{:else if draft_only}
						Never deployed and is only a draft
					{:else}
						Is deployed and has a draft
					{/if}

					{#if onlyOwnDraft}
						<br />
						<span class="text-tertiary italic">Only you can see this {kindLabel}.</span>
					{/if}
				</p>
				{#if draft_users.length > 0}
					<ul class="flex flex-col divide-y border-t">
						{#each orderedUsers as u, i (i)}
							{@const isSelf = !!currentUsername && u.username === currentUsername}
							<li class="flex items-center gap-2 py-1.5">
								<span
									class="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[9px] font-semibold text-white {colorFor(
										u
									)}"
								>
									{initials(u)}
								</span>
								<span class="flex-1 truncate text-primary inline-flex items-center gap-1">
									{fullLabel(u)}{isSelf ? ' (you)' : ''}
									{#if !u.username}
										<Tooltip small>
											{#snippet text()}
												A legacy draft predates the per-user drafts migration: it isn't tied to any
												user (workspace-level, email NULL), so everyone with access to this path
												sees it.
											{/snippet}
										</Tooltip>
									{/if}
								</span>
								{#if actionsEnabled && isSelf}
									<!-- Own draft: jump straight into the editor. -->
									{#if !$userStore?.operator && itemKind && path}
										<Button
											variant="subtle"
											size="xs3"
											startIcon={{ icon: Pencil }}
											on:click={() => goto(editRouteFor(itemKind, path))}
										>
											Edit
										</Button>
									{/if}
								{:else if actionsEnabled && !isSelf}
									{#if !draft_only}
										<Button
											variant="subtle"
											size="xs3"
											startIcon={{ icon: GitCompareArrows }}
											disabled={busyFor !== null && busyFor !== ownerKey(u)}
											loading={busyFor === ownerKey(u)}
											on:click={() => viewDiff(u)}
										>
											View Diff
										</Button>
									{/if}
									<!-- Operators can't edit items, so Load is hidden (View Diff stays, it's
									     read-only). `allowFork=false` (deploy/review page) hides it too. -->
									{#if allowFork && !$userStore?.operator}
										<Button
											variant="subtle"
											size="xs3"
											startIcon={{ icon: Pencil }}
											disabled={busyFor !== null && busyFor !== ownerKey(u)}
											loading={busyFor === ownerKey(u)}
											on:click={() => load(u)}
										>
											Load
										</Button>
									{/if}
									{#if !u.username && canMigrateLegacy}
										<Button
											variant="subtle"
											size="xs3"
											startIcon={{ icon: Wrench }}
											on:click={() => {
												popoverOpen = false
												migrateOpen = true
											}}
										>
											Migrate
										</Button>
									{/if}
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/snippet}
	</Popover>
{/if}

<DiffDrawer bind:this={diffDrawer} isFlow={itemKind === 'flow'} />

{#if workspace && itemKind && path}
	<MigrateLegacyDraftModal
		bind:isOpen={migrateOpen}
		{workspace}
		{itemKind}
		{path}
		ownDraftExists={!!currentUsername && draft_users.some((u) => u.username === currentUsername)}
		onMigrated={() => onMigrated?.()}
	/>
{/if}
