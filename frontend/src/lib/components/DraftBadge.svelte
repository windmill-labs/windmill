<script lang="ts">
	/**
	 * Home-page badge that surfaces "this entity has a draft" plus tiny
	 * user-initial circles for every workspace user with a per-user draft
	 * at this path. Up to 3 circles render inline; with 4+ users we show
	 * the first 2 + a `+N` overflow circle so the badge stays compact.
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

	type DraftUser = { username?: string | null }

	interface Props {
		is_draft?: boolean
		draft_only?: boolean
		draft_users?: DraftUser[]
	}

	let { is_draft = false, draft_only = false, draft_users = [] }: Props = $props()

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
	const MAX_CIRCLES = 3
	const visibleUsers = $derived(
		draft_users.length <= MAX_CIRCLES ? draft_users : draft_users.slice(0, MAX_CIRCLES - 1)
	)
	const overflowCount = $derived(
		draft_users.length > MAX_CIRCLES ? draft_users.length - (MAX_CIRCLES - 1) : 0
	)

	const showBadge = $derived(is_draft || draft_users.length > 0)
</script>

{#if showBadge}
	<Popover notClickable>
		{#snippet text()}
			{#if draft_users.length > 0}
				{draft_only ? 'Never deployed — only a draft exists.' : 'Deployed with drafts pending.'}
				<div class="mt-1 flex flex-col gap-0.5">
					{#each draft_users as u}
						<span>• {fullLabel(u)}</span>
					{/each}
				</div>
			{:else if draft_only}
				Never deployed and is only a draft
			{:else}
				Is deployed and has a draft
			{/if}
		{/snippet}
		<Badge small color="indigo">
			{#if draft_users.length > 0}
				<!-- Circles sit inside the Badge, before the label. `-space-x-1`
				     overlaps them slightly; each circle's ring uses the badge's
				     indigo tint instead of plain white so the overlap reads as
				     intentional rather than a stack-of-floating-dots. -->
				<span class="flex -space-x-1">
					{#each visibleUsers as u}
						<span
							class="inline-flex h-3.5 w-3.5 items-center justify-center rounded-full text-[8px] font-semibold text-white ring-1 ring-indigo-100 dark:ring-indigo-700/40 {colorFor(
								u
							)}"
							title={fullLabel(u)}
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
