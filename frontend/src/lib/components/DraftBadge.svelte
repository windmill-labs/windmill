<script lang="ts">
	/**
	 * Home-page draft badge with per-user initial circles. Hover popover lists
	 * each draft owner; when full context (workspace + itemKind + path) is
	 * passed, OTHER users' rows get inline "View JSON" / "Fork" (forking
	 * yourself is meaningless, so own rows don't). draft_only → "Draft only"
	 * (no deployed row), else "Draft". Renders nothing when there's no draft.
	 */
	import Popover from './meltComponents/Popover.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { Badge } from './common'
	import Button from './common/button/Button.svelte'
	import Modal2 from './common/modal/Modal2.svelte'
	import { Braces, GitFork } from 'lucide-svelte'
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { forkDraftToImport } from '$lib/components/forkDraftToImport'
	import { userStore } from '$lib/stores'

	type DraftUser = { username?: string | null }

	interface Props {
		is_draft?: boolean
		draft_only?: boolean
		draft_users?: DraftUser[]
		/** Authed user's username — pins their circle first and annotates `(you)`. */
		currentUsername?: string | null
		/** Context for the View JSON / Fork actions. Missing → text-only popover. */
		workspace?: string
		itemKind?: UserDraftItemKind
		path?: string
	}

	let {
		is_draft = false,
		draft_only = false,
		draft_users = [],
		currentUsername = undefined,
		workspace = undefined,
		itemKind = undefined,
		path = undefined
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
	let jsonOpen = $state(false)
	let jsonOwnerLabel = $state('')
	let jsonValue = $state<unknown>(undefined)

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

	async function viewJson(owner: DraftUser) {
		busyFor = ownerKey(owner)
		try {
			jsonValue = await fetchDraft(owner)
			jsonOwnerLabel = fullLabel(owner)
			jsonOpen = true
		} catch (e: any) {
			sendUserToast(`Could not load draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}

	async function fork(owner: DraftUser) {
		if (!workspace || !itemKind || !path) return
		busyFor = ownerKey(owner)
		try {
			const value = await fetchDraft(owner)
			// Seed a brand-new own item from the fetched value (no server save).
			forkDraftToImport(itemKind, value, path)
		} catch (e: any) {
			sendUserToast(`Could not fork draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}
</script>

{#if showBadge}
	<Popover openOnHover={true} debounceDelay={50} enableFlyTransition>
		{#snippet trigger()}
			<Badge small color="indigo">
				{#if orderedUsers.length > 0}
					<!-- `-space-x-1` overlaps the circles; the indigo ring tint makes the overlap read intentional. -->
					<span class="flex -space-x-1">
						{#each visibleUsers as u, i (i)}
							<span
								class="inline-flex h-3.5 w-3.5 items-center justify-center rounded-full text-[8px] font-semibold text-white ring-1 ring-indigo-100 dark:ring-indigo-700/40 {colorFor(
									u
								)}"
								title={u.username === currentUsername ? `${fullLabel(u)} (you)` : fullLabel(u)}
							>
								{initials(u)}
							</span>
						{/each}
						{#if overflowCount > 0}
							<span
								class="inline-flex h-3.5 w-3.5 items-center justify-center rounded-full bg-gray-500 text-[8px] font-semibold text-white ring-1 ring-indigo-100 dark:ring-indigo-700/40"
								title="{overflowCount} more"
							>
								+{overflowCount}
							</span>
						{/if}
					</span>
				{/if}
				{draft_only ? 'Draft only' : 'Draft'}
			</Badge>
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
								{#if actionsEnabled && !isSelf}
									<Button
										variant="subtle"
										size="xs3"
										startIcon={{ icon: Braces }}
										disabled={busyFor !== null && busyFor !== ownerKey(u)}
										loading={busyFor === ownerKey(u)}
										on:click={() => viewJson(u)}
									>
										View JSON
									</Button>
									<!-- Operators can't create items, so Fork is hidden (View JSON stays, it's read-only). -->
									{#if !$userStore?.operator}
										<Button
											variant="subtle"
											size="xs3"
											startIcon={{ icon: GitFork }}
											disabled={busyFor !== null && busyFor !== ownerKey(u)}
											loading={busyFor === ownerKey(u)}
											on:click={() => fork(u)}
										>
											Fork
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

<Modal2
	bind:isOpen={jsonOpen}
	title="Draft JSON — {jsonOwnerLabel}"
	fixedWidth="lg"
	fixedHeight="lg"
>
	{#snippet headerRight()}
		<Button
			variant="default"
			size="xs"
			on:click={() => {
				navigator.clipboard?.writeText(JSON.stringify(jsonValue, null, 2))
				sendUserToast('Copied to clipboard')
			}}
		>
			Copy
		</Button>
	{/snippet}
	<div class="w-full overflow-auto">
		<pre class="text-xs whitespace-pre font-mono bg-surface-secondary rounded p-3"
			>{JSON.stringify(jsonValue ?? {}, null, 2)}</pre
		>
	</div>
</Modal2>
