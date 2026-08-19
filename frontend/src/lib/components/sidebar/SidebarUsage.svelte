<script lang="ts">
	import { untrack } from 'svelte'
	import { goto } from '$lib/navigation'
	import { isCloudHosted } from '$lib/cloud'
	import { UserService } from '$lib/gen'
	import {
		isPremiumStore,
		usageStore,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore,
		type UserWorkspace
	} from '$lib/stores'
	import { findWorkspaceAncestors } from '$lib/utils/workspaceHierarchy'
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { EXECUTIONS_HINT, FREE_EXECUTION_QUOTA, SEAT_EXECUTION_QUOTA } from './executionsHint'

	interface Props {
		isCollapsed?: boolean
	}

	let { isCollapsed = false }: Props = $props()

	let open = $state(false)

	// Seat count for a paid workspace, the basis of its included executions. Only
	// the user list is needed for it: `premium_info` carries the same usage number
	// as `workspaceUsageStore` but requires admin and only exists when Stripe is
	// configured, so it would leave regular members with no block at all.
	let seats = $state<number | undefined>(undefined)

	// A fork's usage, tier and bill all resolve to its billing root — the topmost
	// parentless workspace — while its own member list is deliberately a subset of
	// the root's. Counting fork members would meter root usage against a fork-sized
	// cap and invent overages, so seats come from the root. When the root is not
	// visible from here the cap is unknowable and the paid meter stays hidden.
	function billingRoot(workspace: string, all: UserWorkspace[]): string | undefined {
		const self = all.find((w) => w.id === workspace)
		if (!self) return undefined
		if (!self.parent_workspace_id) return workspace
		const top = findWorkspaceAncestors(workspace, all).at(-1)
		return top && !top.parent_workspace_id ? top.id : undefined
	}

	$effect(() => {
		const workspace = $workspaceStore
		const premium = $isPremiumStore
		const all = $userWorkspaces
		untrack(() => {
			seats = undefined
			if (!isCloudHosted() || !premium || !workspace) return
			const root = billingRoot(workspace, all ?? [])
			if (root) loadSeats(workspace, root)
		})
	})

	async function loadSeats(workspace: string, root: string) {
		try {
			// Throws for a fork member with no seat in the root, which is the same
			// answer as an unresolvable root: leave the paid meter hidden.
			const users = await UserService.listUsers({ workspace: root })
			// Nothing cancels the request for the workspace we left, so a slow response
			// for it must not overwrite the one we are on — it would leave the cap wrong
			// until the next switch.
			if ($workspaceStore !== workspace) return
			const developers = users.filter((u) => !u.operator).length
			const operators = users.length - developers
			// Billing-page seat math: 1 developer = 1 seat, 2 operators = 1 seat.
			seats = Math.ceil(developers + operators / 2)
		} catch (e) {
			console.error('Could not compute billing-workspace seats', e)
		}
	}

	type Quota = {
		key: string
		label: string
		short: string
		used: number
		cap: number
		/** Reaching the cap stops jobs, rather than adding to the bill. */
		hard: boolean
	}

	// Free tier: two caps apply at once and either one stops jobs on its own. Paid:
	// one soft cap, the executions the workspace's seats already include.
	const quotas = $derived<Quota[]>(
		$isPremiumStore === undefined
			? []
			: $isPremiumStore
				? seats
					? [
							{
								key: 'workspace',
								label: 'Workspace executions',
								short: 'Workspace execs',
								used: $workspaceUsageStore,
								cap: seats * SEAT_EXECUTION_QUOTA,
								hard: false
							}
						]
					: []
				: [
						{
							key: 'user',
							label: 'Your executions',
							short: 'Your execs',
							used: $usageStore,
							cap: FREE_EXECUTION_QUOTA,
							hard: true
						},
						// The demo workspace has no workspace-level quota.
						...($workspaceStore !== 'demo'
							? [
									{
										key: 'workspace',
										label: 'Workspace executions',
										short: 'Workspace execs',
										used: $workspaceUsageStore,
										cap: FREE_EXECUTION_QUOTA,
										hard: true
									}
								]
							: [])
					]
	)

	const tightest = $derived(
		quotas.length ? quotas.reduce((a, b) => (b.used / b.cap > a.used / a.cap ? b : a)) : undefined
	)

	const FILL = {
		over: { bar: 'bg-red-500', ring: 'stroke-red-500' },
		near: { bar: 'bg-yellow-500', ring: 'stroke-yellow-500' },
		ok: { bar: 'bg-surface-accent-primary', ring: 'stroke-surface-accent-primary' }
	}

	function fill(q: Quota) {
		// Passing a soft cap only means extra billed seats, never a stop, so it
		// never reaches the red the free tier's hard cap gets.
		if (q.used >= q.cap) return q.hard ? FILL.over : FILL.near
		if (q.used >= q.cap * 0.9) return FILL.near
		return FILL.ok
	}

	function ratio(q: Quota): number {
		return q.cap > 0 ? Math.min(q.used / q.cap, 1) : 0
	}

	const fmt = (value: number) => value.toLocaleString('en-US')

	// r=6 with a 2-wide stroke keeps the outer edge at 7, inside the 16x16 box.
	const RING_RADIUS = 6
	const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS
</script>

{#snippet bar(q: Quota)}
	<div class="w-full h-1 rounded-full bg-surface-sunken overflow-hidden">
		<div class="h-full rounded-full {fill(q).bar}" style="width: {ratio(q) * 100}%"></div>
	</div>
{/snippet}

{#snippet ring(q: Quota)}
	<svg viewBox="0 0 16 16" class="h-4 w-4" aria-hidden="true">
		<circle
			cx="8"
			cy="8"
			r={RING_RADIUS}
			fill="none"
			stroke-width="2"
			class="stroke-surface-sunken"
		/>
		<!-- Rotated in SVG user space, not with a CSS transform: the arc is rasterized
		     already turned, so its edges stay crisp at this size. -->
		<circle
			cx="8"
			cy="8"
			r={RING_RADIUS}
			fill="none"
			stroke-width="2"
			transform="rotate(-90 8 8)"
			stroke-dasharray={RING_CIRCUMFERENCE}
			stroke-dashoffset={RING_CIRCUMFERENCE * (1 - ratio(q))}
			class={fill(q).ring}
		/>
	</svg>
{/snippet}

{#if isCloudHosted() && tightest}
	<div class="px-2 pt-2 pb-2">
		<Tooltip placement="right" class="w-full">
			{#snippet text()}
				{tightest.label} this month: {fmt(tightest.used)}/{fmt(tightest.cap)}.
				{EXECUTIONS_HINT}
			{/snippet}
			<button
				class="w-full rounded p-1.5 hover:bg-surface-hover flex {isCollapsed
					? 'justify-center'
					: 'flex-col gap-1'}"
				onclick={() => (open = true)}
				aria-label="{tightest.label} this month: {fmt(tightest.used)} of {fmt(tightest.cap)}"
			>
				{#if isCollapsed}
					{@render ring(tightest)}
				{:else}
					<div
						class="w-full flex items-baseline justify-between gap-2 text-2xs font-normal text-secondary"
					>
						<span class="truncate">{tightest.short}</span>
						<span class="shrink-0 tabular-nums">{fmt(tightest.used)}/{fmt(tightest.cap)}</span>
					</div>
					{@render bar(tightest)}
				{/if}
			</button>
		</Tooltip>
	</div>

	<Modal title="Executions this month" bind:open cancelText="Close">
		<div class="flex flex-col gap-4">
			{#each quotas as quota (quota.key)}
				<div class="flex flex-col gap-1.5">
					<div class="flex items-baseline justify-between gap-2">
						<span class="text-sm text-emphasis">{quota.label}</span>
						<span class="text-sm text-secondary tabular-nums"
							>{fmt(quota.used)}/{fmt(quota.cap)}</span
						>
					</div>
					{@render bar(quota)}
				</div>
			{/each}
			<p class="text-xs text-secondary">
				{EXECUTIONS_HINT} Counters reset at the start of every calendar month.
			</p>
			{#if $isPremiumStore}
				<p class="text-xs text-secondary">
					Your {seats} seat{seats === 1 ? '' : 's'} include {fmt(
						(seats ?? 0) * SEAT_EXECUTION_QUOTA
					)} executions per month. Every extra {fmt(SEAT_EXECUTION_QUOTA)} executions beyond that add
					one billed seat for the month.
				</p>
			{:else}
				<p class="text-xs text-secondary">
					Either quota reaching {fmt(FREE_EXECUTION_QUOTA)} stops jobs from running for the rest of the
					month. Team and Enterprise plans lift both limits.
					{#if !$userStore?.is_admin}
						Ask a workspace admin to change the plan.
					{/if}
				</p>
			{/if}
		</div>
		{#snippet actions()}
			{#if $userStore?.is_admin}
				<Button
					size="sm"
					onclick={() => {
						open = false
						goto('/workspace_settings?tab=premium')
					}}
				>
					{$isPremiumStore ? 'See billing' : 'See plans'}
				</Button>
			{/if}
		{/snippet}
	</Modal>
{/if}
