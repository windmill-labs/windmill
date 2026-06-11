<script lang="ts">
	/**
	 * Home-page badge that surfaces "this entity has a draft" plus tiny
	 * user-initial circles for every workspace user with a per-user draft
	 * at this path. Up to 3 circles render inline; with 4+ users we show
	 * the first 2 + a `+N` overflow circle so the badge stays compact.
	 *
	 * Popover (hover): one row per draft owner with a circle icon. When
	 * the row knows the item kind (workspace + itemKind + path passed
	 * through) each OTHER user's row also gets "View JSON" / "Fork"
	 * buttons — same actions as the in-editor OtherUsersDraftsModal,
	 * surfaced inline here so users don't need to
	 * open the editor first. The authed user's own row never has those
	 * actions (forking yourself is meaningless); when the entry is
	 * draft-only AND it's the authed user's own draft, the popover ends
	 * with "Only you can see this {kind}" so the row's privacy is clear.
	 *
	 * Variants:
	 *   draft_only=true  → "Draft only" (no deployed row exists)
	 *   draft_only=false → "Draft"      (deployed and at least one user
	 *                                    has a draft on top)
	 *
	 * Nothing renders when neither `is_draft` is true nor `draft_users`
	 * is non-empty — the list endpoint omits `draft_users` for paths
	 * with no drafts, so a falsy/empty array is the no-draft signal.
	 */
	import Popover from './Popover.svelte'
	import { Badge } from './common'
	import Button from './common/button/Button.svelte'
	import Modal2 from './common/modal/Modal2.svelte'
	import { Braces, GitFork } from 'lucide-svelte'
	import { DraftService, type UserDraftItemKind } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { forkDraftToImport } from '$lib/components/forkDraftToImport'

	type DraftUser = { username?: string | null }

	interface Props {
		is_draft?: boolean
		draft_only?: boolean
		draft_users?: DraftUser[]
		/** Authed user's workspace username. Used to pin THIS user's
		 *  circle to the first slot (so the authed user always shows
		 *  up front when they have a draft) and to annotate the
		 *  popover entry with `(you)`. Pass `$userStore?.username`
		 *  from the row. */
		currentUsername?: string | null
		/** Optional context needed to render the per-user View JSON /
		 *  Fork actions. When any of these is missing the popover falls
		 *  back to the legacy text-only list (used by trigger rows that
		 *  don't have a per-user draft surface). */
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

	// Authed user always lands FIRST in the circle row when they have a
	// draft; everyone else keeps the backend's alphabetical ordering
	// behind them. The asterisk on the row's summary still flags the
	// own-draft case textually — the leading circle is the visual half
	// of the same signal.
	const orderedUsers = $derived.by(() => {
		if (!currentUsername) return draft_users
		const selfIdx = draft_users.findIndex((u) => u.username === currentUsername)
		if (selfIdx < 0) return draft_users
		const self = draft_users[selfIdx]
		const rest = draft_users.filter((_, i) => i !== selfIdx)
		return [self, ...rest]
	})

	/** Two-letter uppercase initials from a username — `john.doe`/`john_doe` →
	 * `JD`, `alice` → `AL`, the legacy NULL-email row (no username) → `?`. */
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

	// Deterministic color per username so the same user gets the same circle
	// across rows. Tailwind palette of 6 — small enough to read at a glance.
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

	// First 3 circles when ≤3 users; first 2 + a "+N" overflow when 4+.
	// Slice from `orderedUsers` so the authed user — always the first
	// element when present — is always kept (it would be wrong to drop
	// their own draft into the +N bubble while showing strangers).
	const MAX_CIRCLES = 3
	const visibleUsers = $derived(
		orderedUsers.length <= MAX_CIRCLES ? orderedUsers : orderedUsers.slice(0, MAX_CIRCLES - 1)
	)
	const overflowCount = $derived(
		orderedUsers.length > MAX_CIRCLES ? orderedUsers.length - (MAX_CIRCLES - 1) : 0
	)

	// Show the badge whenever ANY draft exists (`draft_users` non-empty)
	// OR when the authed user has a draft (`is_draft` true — the list
	// endpoint sets this even for paths the user has a draft on but no
	// one else does).
	const showBadge = $derived(is_draft || draft_users.length > 0)

	// The popover renders inline actions only when the parent supplied
	// the full context (we need workspace + itemKind + path to actually
	// fetch and fork drafts).
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
		if (!workspace || !itemKind) return
		busyFor = ownerKey(owner)
		try {
			const value = await fetchDraft(owner)
			// Import-style handoff: seed a brand-new own item from the
			// fetched value (no immediate server save, fresh owned path).
			forkDraftToImport(itemKind, value)
		} catch (e: any) {
			sendUserToast(`Could not fork draft: ${e.body ?? e.message}`, true)
		} finally {
			busyFor = null
		}
	}
</script>

{#if showBadge}
	<Popover notClickable>
		{#snippet text()}
			<div class="flex flex-col gap-2 min-w-[16rem]">
				<p class="text-primary">
					{#if draft_users.length > 0}
						{draft_only ? 'Never deployed — only a draft exists.' : 'Deployed with drafts pending.'}
					{:else if draft_only}
						Never deployed and is only a draft
					{:else}
						Is deployed and has a draft
					{/if}
				</p>
				{#if draft_users.length > 0}
					<ul class="flex flex-col divide-y border-y">
						<!-- Key on the array index, NOT the username. The list is
						     short-lived (the popover lifetime) so we don't need
						     stable reordering, and multiple legacy NULL-email rows
						     could otherwise collide on the same key and crash the
						     page (the `draft_users` aggregate LEFT-JOINs `usr`, so
						     drafts owned by users absent from the workspace surface
						     as `username: null` too — not just the legacy row). -->
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
								<span class="flex-1 truncate text-primary">
									{fullLabel(u)}{isSelf ? ' (you)' : ''}
								</span>
								{#if actionsEnabled && !isSelf}
									<Button
										variant="default"
										size="xs"
										startIcon={{ icon: Braces }}
										disabled={busyFor !== null && busyFor !== ownerKey(u)}
										loading={busyFor === ownerKey(u)}
										on:click={() => viewJson(u)}
									>
										View JSON
									</Button>
									<Button
										variant="default"
										size="xs"
										startIcon={{ icon: GitFork }}
										disabled={busyFor !== null && busyFor !== ownerKey(u)}
										loading={busyFor === ownerKey(u)}
										on:click={() => fork(u)}
									>
										Fork
									</Button>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
				{#if onlyOwnDraft}
					<p class="text-tertiary italic">Only you can see this {kindLabel}.</p>
				{/if}
			</div>
		{/snippet}
		<Badge small color="indigo">
			{#if orderedUsers.length > 0}
				<!-- Circles sit inside the Badge, before the label. `-space-x-1`
				     overlaps them slightly; each circle's ring uses the badge's
				     indigo tint instead of plain white so the overlap reads as
				     intentional rather than a stack-of-floating-dots. The
				     authed user is pinned to the first slot when present (see
				     `orderedUsers`). -->
				<span class="flex -space-x-1">
					{#each visibleUsers as u}
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
