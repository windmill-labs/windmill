<script lang="ts">
	// DEV-ONLY design exploration for the AI-chat "jobs tray" (background jobs the
	// chat started). Goal: align the visual with the runs page (Badge + icon +
	// color), stay small/minimal, and handle richer states — queued, running,
	// waiting-for-approval (suspend), scheduled, success, failure, canceled — with
	// "open in preview" for anything that needs more than a row. Not linked
	// anywhere; open at /dev/jobs-tray. Pure mock data. Gated behind the dev flag.
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import { slide } from 'svelte/transition'
	import {
		Ban,
		Calendar,
		Check,
		ChevronRight,
		Clock,
		ExternalLink,
		Hourglass,
		Play,
		ThumbsUp,
		TimerOff,
		Trash2,
		X
	} from 'lucide-svelte'

	type Status =
		| 'queued'
		| 'running'
		| 'suspended'
		| 'scheduled'
		| 'success'
		| 'failure'
		| 'canceled'

	type Kind = 'script' | 'flow' | 'step'

	type Job = {
		id: string
		label: string
		kind: Kind
		status: Status
		createdAt: number
		durationMs?: number
	}

	const SAMPLE_LABELS: Record<Kind, string[]> = {
		script: ['u/admin/backfill_accounts', 'f/etl/validate_schema', 'u/alice/export_report'],
		flow: ['f/etl/nightly_pipeline', 'u/admin/deploy_service', 'f/ops/reconcile'],
		step: ['step transform', 'step fetch_rows', 'step notify']
	}

	// Persist the exploration config to localStorage so an HMR reload (frequent
	// while iterating on this page) doesn't wipe the jobs/collapse state.
	const STORAGE_KEY = 'wm_dev_jobs_tray'
	type Saved = {
		seq: number
		jobs: Job[]
		open: { a: boolean; b: boolean; c: boolean; d: boolean }
	}
	function loadSaved(): Saved | null {
		if (typeof localStorage === 'undefined') return null
		try {
			const raw = localStorage.getItem(STORAGE_KEY)
			return raw ? (JSON.parse(raw) as Saved) : null
		} catch {
			return null
		}
	}
	const saved = loadSaved()

	let seq = $state(saved?.seq ?? 0)
	let jobs = $state<Job[]>(saved?.jobs ?? [])
	// Per-variant collapsed/expanded state, so the collapsed presentation can be
	// explored (and compared) for each design.
	let open = $state(saved?.open ?? { a: true, b: true, c: true, d: true })
	function setAllOpen(v: boolean) {
		open = { a: v, b: v, c: v, d: v }
	}

	// Variant D: cap to the 5 most recent, "Show more" reveals older ones.
	const PAGE_SIZE = 5
	let visibleCountD = $state(PAGE_SIZE)
	const sortedJobsD = $derived([...jobs].sort((a, b) => b.createdAt - a.createdAt))
	const visibleJobsD = $derived(sortedJobsD.slice(0, visibleCountD))
	const hiddenCountD = $derived(Math.max(0, sortedJobsD.length - visibleCountD))

	$effect(() => {
		if (typeof localStorage === 'undefined') return
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify({ seq, jobs, open }))
		} catch {
			// ignore quota/serialization errors — this is dev-only convenience
		}
	})

	// Live clock for elapsed times, only ticking while something is in flight.
	let now = $state(Date.now())
	const hasLive = $derived(
		jobs.some((j) => j.status === 'running' || j.status === 'queued' || j.status === 'suspended')
	)
	$effect(() => {
		if (!hasLive) return
		const t = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(t)
	})

	function pick<T>(arr: T[], i: number): T {
		return arr[i % arr.length]
	}

	function addJob(status: Status, kind: Kind = 'script') {
		seq += 1
		const terminal = status === 'success' || status === 'failure' || status === 'canceled'
		jobs = [
			...jobs,
			{
				id: `mock-${seq}`,
				label: pick(SAMPLE_LABELS[kind], seq),
				kind,
				status,
				createdAt: Date.now() - (terminal ? 30000 + seq * 4000 : seq * 3000),
				durationMs: terminal ? 5000 + (seq % 7) * 8000 : undefined
			}
		]
	}

	function addSampleSet() {
		clearAll()
		addJob('running', 'flow')
		addJob('queued', 'script')
		addJob('suspended', 'flow')
		addJob('scheduled', 'script')
		addJob('success', 'script')
		addJob('failure', 'step')
		addJob('canceled', 'flow')
	}

	function clearAll() {
		jobs = []
	}

	function removeJob(id: string) {
		jobs = jobs.filter((j) => j.id !== id)
	}

	// Advance one step in a plausible lifecycle, for eyeballing transitions.
	function advance(id: string) {
		jobs = jobs.map((j) => {
			if (j.id !== id) return j
			const next: Record<Status, Status> = {
				queued: 'running',
				scheduled: 'running',
				running: 'suspended',
				suspended: 'success',
				success: 'success',
				failure: 'failure',
				canceled: 'canceled'
			}
			const status = next[j.status]
			const terminal = status === 'success'
			return { ...j, status, durationMs: terminal ? now - j.createdAt : j.durationMs }
		})
	}

	function cancel(id: string) {
		jobs = jobs.map((j) =>
			j.id === id ? { ...j, status: 'canceled', durationMs: now - j.createdAt } : j
		)
	}

	function approve(id: string) {
		jobs = jobs.map((j) => (j.id === id ? { ...j, status: 'running' } : j))
	}

	function isTerminal(status: Status): boolean {
		return status === 'success' || status === 'failure' || status === 'canceled'
	}
	function isApproval(status: Status): boolean {
		return status === 'suspended'
	}

	function elapsed(job: Job): string {
		const ms = isTerminal(job.status) ? (job.durationMs ?? 0) : now - job.createdAt
		const total = Math.max(0, Math.floor(ms / 1000))
		const m = Math.floor(total / 60)
		const s = total % 60
		return `${m}:${s.toString().padStart(2, '0')}`
	}

	// Runs-page vocabulary (mirrors JobStatusIcon.svelte).
	type BadgeMeta = { color: any; icon: any; text: string; spin?: boolean }
	function badgeMeta(status: Status): BadgeMeta {
		switch (status) {
			case 'running':
				return { color: 'yellow', icon: Play, text: 'Running' }
			case 'suspended':
				return { color: 'violet', icon: Hourglass, text: 'Approval' }
			case 'scheduled':
				return { color: 'blue', icon: Calendar, text: 'Scheduled' }
			case 'success':
				return { color: 'green', icon: Check, text: 'Success' }
			case 'failure':
				return { color: 'red', icon: X, text: 'Failed' }
			case 'canceled':
				return { color: 'gray', icon: Ban, text: 'Canceled' }
			default:
				return { color: 'orange', icon: Clock, text: 'Queued' }
		}
	}

	const running = $derived(jobs.filter((j) => j.status === 'running').length)
	const queued = $derived(jobs.filter((j) => j.status === 'queued').length)
	const approvals = $derived(jobs.filter((j) => j.status === 'suspended').length)

	const enabled = isGlobalAiEnabled()
</script>

<div class="min-h-screen bg-surface-secondary p-6 text-primary">
	<div class="mx-auto max-w-5xl">
		<header class="mb-6">
			<h1 class="text-xl font-semibold">Jobs tray — design kitchen sink</h1>
			<p class="mt-1 max-w-2xl text-sm text-secondary">
				Explore how the AI chat should list background jobs. Same status vocabulary as the runs page
				(Badge + icon + color), kept minimal — anything needing more detail opens in the preview
				pane. Add jobs in any state on the left; every variant renders the same list.
			</p>
			{#if !enabled}
				<p class="mt-2 text-xs text-orange-600">
					Dev flag off — set <code>localStorage.wm_dev_global_ai = '1'</code> and reload to mirror the
					real gate (this page still works regardless).
				</p>
			{/if}
		</header>

		<div class="grid grid-cols-1 gap-6 md:grid-cols-[240px_1fr]">
			<!-- Controls -->
			<aside class="h-fit md:sticky md:top-6">
				<div class="rounded-lg border bg-surface p-4">
					<div class="mb-2 text-xs font-semibold uppercase tracking-wide text-tertiary">
						Add job
					</div>
					<div class="flex flex-wrap gap-1.5">
						<Button
							size="xs"
							variant="border"
							color="yellow"
							on:click={() => addJob('running', 'flow')}>Running</Button
						>
						<Button size="xs" variant="border" on:click={() => addJob('queued', 'script')}
							>Queued</Button
						>
						<Button
							size="xs"
							variant="border"
							color="violet"
							on:click={() => addJob('suspended', 'flow')}>Approval</Button
						>
						<Button
							size="xs"
							variant="border"
							color="blue"
							on:click={() => addJob('scheduled', 'script')}>Scheduled</Button
						>
						<Button
							size="xs"
							variant="border"
							color="green"
							on:click={() => addJob('success', 'script')}>Success</Button
						>
						<Button
							size="xs"
							variant="border"
							color="red"
							on:click={() => addJob('failure', 'step')}>Failed</Button
						>
						<Button
							size="xs"
							variant="border"
							color="gray"
							on:click={() => addJob('canceled', 'flow')}>Canceled</Button
						>
					</div>

					<div class="my-4 border-t"></div>

					<div class="flex flex-col gap-1.5">
						<Button size="xs" variant="contained" on:click={addSampleSet}>Add sample set</Button>
						<Button
							size="xs"
							variant="border"
							color="red"
							startIcon={{ icon: Trash2 }}
							on:click={clearAll}>Clear all</Button
						>
					</div>

					<div class="my-4 border-t"></div>

					<div class="mb-1.5 text-xs font-semibold uppercase tracking-wide text-tertiary">
						Collapsed preview
					</div>
					<div class="flex gap-1.5">
						<Button size="xs" variant="border" on:click={() => setAllOpen(false)}
							>Collapse all</Button
						>
						<Button size="xs" variant="border" on:click={() => setAllOpen(true)}>Expand all</Button>
					</div>
					<p class="mt-1.5 text-2xs text-tertiary">
						Or click any variant's header to toggle it individually.
					</p>

					<div class="my-4 border-t"></div>

					<div class="text-xs text-secondary">
						<div class="flex justify-between"><span>Total</span><span>{jobs.length}</span></div>
						<div class="flex justify-between"><span>Running</span><span>{running}</span></div>
						<div class="flex justify-between"><span>Queued</span><span>{queued}</span></div>
						<div class="flex justify-between"><span>Approvals</span><span>{approvals}</span></div>
					</div>
					<p class="mt-3 text-2xs text-tertiary">
						Tip: hover a row and use ▸ to advance its lifecycle (queued → running → approval →
						success).
					</p>
				</div>
			</aside>

			<!-- Variants -->
			<div class="flex flex-col gap-6">
				{@render variantCard(
					'A · Current',
					"What ships today: a bare colored dot. Doesn't match the runs page and reads flat.",
					variantCurrent
				)}
				{@render variantCard(
					'B · Runs-aligned badge',
					'Same Badge + icon + color as the runs page. Status pill on the left, label + elapsed, actions on hover. Approval gets an inline Approve.',
					variantBadge
				)}
				{@render variantCard(
					'C · Compact single-line',
					'Densest: icon-only status, label truncates, elapsed right-aligned, actions on hover. Best when several jobs stack.',
					variantCompact
				)}
				{@render variantCard(
					'D · B + real detail-page actions',
					'Variant B, but Cancel is the accent/destructive Button from the run detail page and Approve is an accent Button — full, unambiguous actions instead of tiny icons.',
					variantDetailActions
				)}
			</div>
		</div>
	</div>
</div>

<!-- ============ shared snippets ============ -->

{#snippet variantCard(title: string, desc: string, body: any)}
	<section class="rounded-lg border bg-surface">
		<div class="border-b px-4 py-2.5">
			<div class="text-sm font-semibold">{title}</div>
			<div class="text-xs text-tertiary">{desc}</div>
		</div>
		<div class="p-4">
			{#if jobs.length === 0}
				<div class="py-6 text-center text-xs text-tertiary">No jobs — add some on the left.</div>
			{:else}
				{@render body()}
			{/if}
		</div>
	</section>
{/snippet}

<!-- Variant A: current bare-dot design -->
{#snippet variantCurrent()}
	<div class="rounded-md border bg-surface-tertiary text-xs">
		<button
			type="button"
			class="flex w-full items-center gap-2 px-3 py-2"
			onclick={() => (open.a = !open.a)}
		>
			<ChevronRight
				size={14}
				class="text-tertiary transition-transform duration-150 {open.a ? 'rotate-90' : ''}"
			/>
			<span class="text-xs font-normal text-primary">Jobs</span>
			<span class="ml-auto text-tertiary">{jobs.length}</span>
		</button>
		{#if open.a}
			<div transition:slide={{ duration: 150 }}>
				{#each jobs as job (job.id)}
					<div class="flex items-center gap-2 border-t px-3 py-2">
						<span
							class="size-2.5 shrink-0 rounded-full {job.status === 'running'
								? 'bg-blue-500'
								: job.status === 'success'
									? 'bg-green-500'
									: job.status === 'failure'
										? 'bg-red-500'
										: job.status === 'suspended'
											? 'bg-violet-500'
											: 'bg-gray-400'}"
						></span>
						<div class="min-w-0 grow">
							<div class="truncate text-secondary">{job.label}</div>
							<div class="text-tertiary"
								>{job.kind} · {badgeMeta(job.status).text.toLowerCase()}</div
							>
						</div>
						<span class="tabular-nums text-tertiary">{elapsed(job)}</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<!-- Variant B: runs-aligned badge -->
{#snippet variantBadge()}
	<div class="rounded-md border bg-surface-tertiary text-xs">
		<button
			type="button"
			class="flex w-full items-center gap-2 px-3 py-2"
			onclick={() => (open.b = !open.b)}
		>
			<ChevronRight
				size={14}
				class="text-tertiary transition-transform duration-150 {open.b ? 'rotate-90' : ''}"
			/>
			<span class="text-xs font-normal text-primary">Jobs</span>
			<span class="ml-auto flex items-center gap-1">
				{#if running}<Badge color="yellow" small>{running} running</Badge>{/if}
				{#if approvals}<Badge color="violet" small>{approvals} approval</Badge>{/if}
				{#if queued}<Badge color="orange" small>{queued} queued</Badge>{/if}
			</span>
		</button>
		{#if open.b}
			<div transition:slide={{ duration: 150 }}>
				{#each jobs as job (job.id)}
					{@const meta = badgeMeta(job.status)}
					{@const Icon = meta.icon}
					<div class="group flex items-center gap-2.5 border-t px-3 py-2">
						<Badge color={meta.color} baseClass="!px-1.5" title={meta.text}>
							<Icon size={13} />
						</Badge>
						<span class="min-w-0 grow truncate text-secondary" title={job.label}>{job.label}</span>
						<span class="shrink-0 tabular-nums text-tertiary">{elapsed(job)}</span>
						<div class="flex shrink-0 items-center gap-1">
							{#if isApproval(job.status)}
								<Button
									size="xs"
									variant="contained"
									color="violet"
									startIcon={{ icon: ThumbsUp }}
									on:click={() => approve(job.id)}>Approve</Button
								>
							{/if}
							<button
								type="button"
								class="text-tertiary hover:text-primary"
								title="Open in preview"
								onclick={() => advance(job.id)}
							>
								<ExternalLink size={13} />
							</button>
							{#if isTerminal(job.status)}
								<button
									type="button"
									class="text-tertiary opacity-0 hover:text-primary group-hover:opacity-100"
									title="Remove"
									onclick={() => removeJob(job.id)}
								>
									<X size={13} />
								</button>
							{:else}
								<button
									type="button"
									class="text-tertiary opacity-0 hover:text-red-500 group-hover:opacity-100"
									title="Cancel"
									onclick={() => cancel(job.id)}
								>
									<Ban size={13} />
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<!-- Variant C: compact single-line -->
{#snippet variantCompact()}
	<div class="rounded-md border bg-surface-tertiary text-xs">
		<button
			type="button"
			class="flex w-full items-center gap-2 px-3 py-1.5"
			onclick={() => (open.c = !open.c)}
		>
			<ChevronRight
				size={13}
				class="text-tertiary transition-transform duration-150 {open.c ? 'rotate-90' : ''}"
			/>
			<span class="text-xs font-normal text-primary">Jobs</span>
			<span class="ml-auto flex items-center gap-1.5 text-tertiary">
				{#if running}<span class="text-yellow-600">{running} running</span>{/if}
				{#if approvals}<span class="text-violet-600">{approvals} approval</span>{/if}
				{#if queued}<span class="text-orange-600">{queued} queued</span>{/if}
			</span>
		</button>
		{#if open.c}
			<div transition:slide={{ duration: 150 }} class="border-t py-1">
				{#each jobs as job (job.id)}
					{@const meta = badgeMeta(job.status)}
					{@const Icon = meta.icon}
					<div class="group flex items-center gap-2 px-3 py-1.5">
						<Badge color={meta.color} baseClass="!px-1.5" title={meta.text}>
							<Icon size={13} />
						</Badge>
						<span class="truncate text-secondary" title={job.label}>{job.label}</span>
						<span class="ml-auto shrink-0 tabular-nums text-tertiary">{elapsed(job)}</span>
						{#if isApproval(job.status)}
							<Button
								size="xs"
								variant="contained"
								color="violet"
								startIcon={{ icon: ThumbsUp }}
								on:click={() => approve(job.id)}>Approve</Button
							>
						{/if}
						<div
							class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100"
						>
							<button
								type="button"
								class="text-tertiary hover:text-primary"
								title="Open in preview"
								onclick={() => advance(job.id)}
							>
								<ExternalLink size={13} />
							</button>
							{#if isTerminal(job.status)}
								<button
									type="button"
									class="text-tertiary hover:text-primary"
									title="Remove"
									onclick={() => removeJob(job.id)}
								>
									<X size={13} />
								</button>
							{:else}
								<button
									type="button"
									class="text-tertiary hover:text-red-500"
									title="Cancel"
									onclick={() => cancel(job.id)}
								>
									<Ban size={13} />
								</button>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/snippet}

<!-- Variant D: B with the run-detail-page action buttons -->
{#snippet variantDetailActions()}
	<div class="rounded-md border bg-surface-tertiary text-xs">
		<button
			type="button"
			class="flex w-full items-center gap-2 px-3 py-2"
			onclick={() => (open.d = !open.d)}
		>
			<ChevronRight
				size={14}
				class="text-tertiary transition-transform duration-150 {open.d ? 'rotate-90' : ''}"
			/>
			<span class="text-xs font-normal text-primary">Jobs</span>
			<span class="ml-auto flex items-center gap-1">
				{#if running}<Badge color="yellow" small>{running} running</Badge>{/if}
				{#if approvals}<Badge color="violet" small>{approvals} approval</Badge>{/if}
				{#if queued}<Badge color="orange" small>{queued} queued</Badge>{/if}
			</span>
		</button>
		{#if open.d}
			<div transition:slide={{ duration: 150 }} class="border-t py-1">
				{#each visibleJobsD as job (job.id)}
					{@const meta = badgeMeta(job.status)}
					{@const Icon = meta.icon}
					<div class="group flex items-center gap-2.5 px-3 py-2">
						<Badge color={meta.color} baseClass="!px-1.5" title={meta.text}>
							<Icon size={13} />
						</Badge>
						<span class="min-w-0 grow truncate text-secondary" title={job.label}>{job.label}</span>
						<span class="shrink-0 tabular-nums text-tertiary">{elapsed(job)}</span>
						<div class="flex shrink-0 items-center gap-1.5">
							{#if isApproval(job.status)}
								<Button
									unifiedSize="xs"
									variant="accent"
									startIcon={{ icon: ThumbsUp }}
									on:click={() => approve(job.id)}>Approve</Button
								>
							{/if}
							{#if !isTerminal(job.status)}
								<Button
									unifiedSize="xs"
									variant="accent"
									destructive
									startIcon={{ icon: TimerOff }}
									on:click={() => cancel(job.id)}>Cancel</Button
								>
							{/if}
							<button
								type="button"
								class="text-tertiary hover:text-primary"
								title="Open in preview"
								onclick={() => advance(job.id)}
							>
								<ExternalLink size={13} />
							</button>
						</div>
					</div>
				{/each}
				{#if hiddenCountD > 0}
					<button
						type="button"
						class="flex w-full items-center justify-center border-t px-3 py-1.5 text-tertiary hover:text-primary"
						onclick={() => (visibleCountD += PAGE_SIZE)}
					>
						Show more ({hiddenCountD})
					</button>
				{/if}
			</div>
		{/if}
	</div>
{/snippet}
