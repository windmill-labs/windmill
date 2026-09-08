<script lang="ts">
	import { resource } from 'runed'
	import { goto } from '$lib/navigation'
	import { isCloudHosted } from '$lib/cloud'
	import { WorkspaceService } from '$lib/gen'
	import {
		isPremiumStore,
		usageStore,
		userStore,
		workspaceMembershipVersion,
		workspaceStore,
		workspaceUsageStore
	} from '$lib/stores'
	import { refreshExecutions } from '$lib/usage.svelte'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { scopedValue, tagged } from '$lib/utils/scopedValue'
	import { Button } from '$lib/components/common'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Tooltip } from '$lib/components/meltComponents'
	import { EXECUTIONS_HINT, FREE_EXECUTION_QUOTA, SEAT_EXECUTION_QUOTA } from './executionsHint'

	interface Props {
		isCollapsed?: boolean
	}

	let { isCollapsed = false }: Props = $props()

	let open = $state(false)

	// Seat count for a paid workspace, the basis of its included executions. The server
	// resolves a fork to the workspace its plan is billed on and counts the seats there,
	// because neither is answerable from here: a fork's member list is a subset of that
	// root's, and a fork member need not be a member of the root at all.
	const fetchSeats = tagged(
		async (workspace: string) => (await WorkspaceService.getBillableSeats({ workspace })).seats
	)

	const meteredWorkspace = $derived(
		isCloudHosted() && $isPremiumStore ? $workspaceStore : undefined
	)

	// The membership version is in the key so a change re-resolves the cap, but not in
	// the tag: tagging by it would blank the bar on every change.
	const seatsResource = resource(
		() =>
			meteredWorkspace
				? { workspace: meteredWorkspace, version: $workspaceMembershipVersion }
				: undefined,
		async (key) => (key ? await fetchSeats(key.workspace) : undefined)
	)

	const scopedSeats = scopedValue<number>()
	const seats = $derived(scopedSeats(meteredWorkspace, seatsResource.current))

	type QuotaKey = 'user' | 'workspace'

	/** Every key the meter can report, so the whole vocabulary is readable here. A paid
	workspace has no per-user quota, so `paid:user` does not exist. */
	type UsageMeterKey = `free:${QuotaKey}` | 'paid:workspace'

	type Quota = {
		key: QuotaKey
		label: string
		short: string
		used: number
		cap: number
		/** Reaching the cap stops jobs, rather than adding to the bill. */
		hard: boolean
	}

	// Every input is tri-state while it resolves, and a quota built from a missing
	// one would render as a real number: each is listed only once its own usage,
	// tier and cap are known. Free tier: two caps apply at once and either stops
	// jobs on its own. Paid: one soft cap, the executions the seats already include.
	const quotas = $derived<Quota[]>(
		$isPremiumStore === undefined
			? []
			: $isPremiumStore
				? seats !== undefined && $workspaceUsageStore !== undefined
					? [
							{
								key: 'workspace' as const,
								label: 'Workspace executions',
								short: 'Workspace execs',
								used: $workspaceUsageStore,
								cap: seats * SEAT_EXECUTION_QUOTA,
								hard: false
							}
						]
					: []
				: [
						...($usageStore !== undefined
							? [
									{
										key: 'user' as const,
										label: 'Your executions',
										short: 'Your execs',
										used: $usageStore,
										cap: FREE_EXECUTION_QUOTA,
										hard: true
									}
								]
							: []),
						// The demo workspace has no workspace-level quota.
						...($workspaceStore !== 'demo' && $workspaceUsageStore !== undefined
							? [
									{
										key: 'workspace' as const,
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
				type="button"
				class="w-full rounded p-1.5 hover:bg-surface-hover flex {isCollapsed
					? 'justify-center'
					: 'flex-col gap-1'}"
				onclick={() => {
					open = true
					const key: UsageMeterKey = $isPremiumStore ? 'paid:workspace' : `free:${tightest.key}`
					logFeatureUsage('usage_meter', 'opened', { key })
					// Executions accrue continuously and seats move with membership; both
					// are read here rather than glanced at, so re-read both.
					refreshExecutions()
					void seatsResource.refetch()
				}}
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
					unifiedSize="md"
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
